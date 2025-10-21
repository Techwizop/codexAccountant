use std::sync::Arc;

use chrono::DateTime;
use chrono::Utc;
use codex_approvals::ApprovalTask;
use codex_approvals::ApprovalsResult;
use codex_approvals::ApprovalsService;
use codex_approvals::QueueFilter;
use codex_policy::CompanyId;
use codex_policy::PolicyResult;
use codex_policy::PolicyRuleSet;
use codex_policy::PolicyStore;
use serde::Deserialize;
use serde::Serialize;

use crate::AccountingTelemetry;

#[derive(Clone)]
pub struct ControlsFacade {
    policy_store: Arc<dyn PolicyStore>,
    approvals: Arc<dyn ApprovalsService>,
    telemetry: Option<Arc<AccountingTelemetry>>,
}

impl ControlsFacade {
    pub fn new(policy_store: Arc<dyn PolicyStore>, approvals: Arc<dyn ApprovalsService>) -> Self {
        Self::with_telemetry(policy_store, approvals, None)
    }

    pub fn with_telemetry(
        policy_store: Arc<dyn PolicyStore>,
        approvals: Arc<dyn ApprovalsService>,
        telemetry: Option<Arc<AccountingTelemetry>>,
    ) -> Self {
        Self {
            policy_store,
            approvals,
            telemetry,
        }
    }

    pub async fn list_policy_rule_sets(&self) -> PolicyResult<Vec<PolicyRuleSetView>> {
        let rule_sets = self.policy_store.list_rule_sets().await?;
        let mut views = rule_sets
            .into_iter()
            .map(|(company_id, rules)| PolicyRuleSetView { company_id, rules })
            .collect::<Vec<_>>();
        views.sort_by(|a, b| a.company_id.cmp(&b.company_id));
        Ok(views)
    }

    pub async fn approvals_queue(
        &self,
        filter: QueueFilter,
    ) -> ApprovalsResult<ApprovalsQueueView> {
        let mut tasks = self.approvals.list(filter).await?;
        tasks.sort_by(|a, b| a.request.submitted_at.cmp(&b.request.submitted_at));
        let generated_at = Utc::now();
        let overdue = tasks
            .iter()
            .filter(|task| task.is_overdue(generated_at))
            .cloned()
            .collect::<Vec<_>>();
        let view = ApprovalsQueueView {
            generated_at,
            tasks,
            overdue,
        };
        if let Some(telemetry) = &self.telemetry {
            telemetry.record_approvals_snapshot(view.tasks.len(), view.overdue.len());
        }
        Ok(view)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyRuleSetView {
    pub company_id: CompanyId,
    pub rules: PolicyRuleSet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalsQueueView {
    pub generated_at: DateTime<Utc>,
    pub tasks: Vec<ApprovalTask>,
    pub overdue: Vec<ApprovalTask>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_approvals::ApprovalDecision;
    use codex_approvals::ApprovalRequest;
    use codex_approvals::ApprovalStage;
    use codex_approvals::DecisionInput;
    use codex_approvals::InMemoryApprovalsService;
    use codex_approvals::QueueFilter;
    use codex_policy::InMemoryPolicyStore;
    use codex_policy::PolicyRuleSet;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn facade_lists_policies_and_queue() {
        let store = Arc::new(InMemoryPolicyStore::new());
        store
            .put_rule_set("comp-1".into(), PolicyRuleSet::default())
            .await
            .expect("policy insert");
        let policy_store: Arc<dyn PolicyStore> = store;

        let approvals = Arc::new(InMemoryApprovalsService::new());
        let approvals_service: Arc<dyn ApprovalsService> = approvals.clone();
        let facade = ControlsFacade::new(policy_store, approvals_service.clone());

        let policies = facade
            .list_policy_rule_sets()
            .await
            .expect("policies should list");
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].company_id, "comp-1");

        let mut request =
            ApprovalRequest::new("comp-1".into(), "user-1".into(), "Multi stage".into());
        request.stages = vec![
            ApprovalStage {
                approvers: vec!["approver-1".into()],
            },
            ApprovalStage {
                approvers: vec!["approver-2".into()],
            },
        ];
        request.sla_at = Some(Utc::now() - chrono::Duration::minutes(10));
        approvals
            .enqueue(request)
            .await
            .expect("enqueue should succeed");

        let queue = facade
            .approvals_queue(QueueFilter {
                company_id: Some("comp-1".into()),
                ..QueueFilter::default()
            })
            .await
            .expect("queue should load");
        assert_eq!(queue.tasks.len(), 1);
        assert_eq!(queue.overdue.len(), 1);

        approvals_service
            .decide(
                &queue.tasks[0].request.id,
                DecisionInput {
                    decided_by: "approver-1".into(),
                    decision: ApprovalDecision::Approved,
                    reason: None,
                },
            )
            .await
            .expect("first stage approve");

        let queue_after = facade
            .approvals_queue(QueueFilter {
                company_id: Some("comp-1".into()),
                ..QueueFilter::default()
            })
            .await
            .expect("queue should load after approval");
        assert_eq!(queue_after.tasks.len(), 1);
        assert_eq!(queue_after.tasks[0].current_stage_index, 1);
    }

    #[tokio::test]
    async fn approvals_queue_records_telemetry_snapshot() {
        let store = Arc::new(InMemoryPolicyStore::new());
        store
            .put_rule_set("comp-telemetry".into(), PolicyRuleSet::default())
            .await
            .expect("policy insert");
        let policy_store: Arc<dyn PolicyStore> = store;

        let approvals = Arc::new(InMemoryApprovalsService::new());
        let approvals_service: Arc<dyn ApprovalsService> = approvals.clone();
        let telemetry = Arc::new(AccountingTelemetry::new());
        let facade = ControlsFacade::with_telemetry(
            policy_store,
            approvals_service.clone(),
            Some(telemetry.clone()),
        );

        let mut overdue_request =
            ApprovalRequest::new("comp-telemetry".into(), "user-1".into(), "Overdue".into());
        overdue_request.stages = vec![ApprovalStage {
            approvers: vec!["approver-a".into()],
        }];
        overdue_request.sla_at = Some(Utc::now() - chrono::Duration::minutes(5));
        approvals
            .enqueue(overdue_request)
            .await
            .expect("enqueue overdue");

        let mut fresh_request =
            ApprovalRequest::new("comp-telemetry".into(), "user-2".into(), "Fresh".into());
        fresh_request.stages = vec![ApprovalStage {
            approvers: vec!["approver-b".into()],
        }];
        fresh_request.sla_at = Some(Utc::now() + chrono::Duration::minutes(30));
        approvals
            .enqueue(fresh_request)
            .await
            .expect("enqueue fresh");

        let queue = facade
            .approvals_queue(QueueFilter {
                company_id: Some("comp-telemetry".into()),
                ..QueueFilter::default()
            })
            .await
            .expect("queue loads with telemetry");
        assert_eq!(queue.tasks.len(), 2);
        assert_eq!(queue.overdue.len(), 1);

        let counters = telemetry.snapshot();
        assert_eq!(counters.approvals_total, 2);
        assert_eq!(counters.approvals_overdue, 1);
    }
}
