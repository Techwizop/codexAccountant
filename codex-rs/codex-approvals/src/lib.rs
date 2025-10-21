#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type ApprovalId = String;
pub type CompanyId = String;
pub type UserId = String;

pub type ApprovalsResult<T> = Result<T, ApprovalsError>;

#[derive(Debug, Error)]
pub enum ApprovalsError {
    #[error("approval {0} was not found")]
    NotFound(String),
    #[error("approval is already assigned to {assignee}")]
    AlreadyAssigned { assignee: UserId },
    #[error("approval is not assigned to {0}")]
    NotAssigned(UserId),
    #[error("approval is finalized and cannot transition")]
    Finalized,
    #[error("validation error: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalStage {
    #[serde(default)]
    pub approvers: Vec<UserId>,
}

impl ApprovalStage {
    #[must_use]
    pub fn allows(&self, user_id: &UserId) -> bool {
        self.approvers.is_empty() || self.approvers.iter().any(|candidate| candidate == user_id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalId,
    pub company_id: CompanyId,
    pub submitted_by: UserId,
    pub submitted_at: DateTime<Utc>,
    pub summary: String,
    pub amount_minor: i64,
    pub currency: String,
    pub priority: ApprovalPriority,
    pub sla_at: Option<DateTime<Utc>>,
    pub metadata: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<ApprovalStage>,
}

impl ApprovalRequest {
    pub fn new(company_id: CompanyId, submitted_by: UserId, summary: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            company_id,
            submitted_by,
            submitted_at: Utc::now(),
            summary,
            amount_minor: 0,
            currency: "USD".into(),
            priority: ApprovalPriority::Normal,
            sla_at: None,
            metadata: None,
            stages: Vec::new(),
        }
    }

    pub fn validate(&self) -> ApprovalsResult<()> {
        if self.summary.trim().is_empty() {
            return Err(ApprovalsError::Validation(
                "approval summary must be provided".into(),
            ));
        }
        if self.currency.trim().is_empty() {
            return Err(ApprovalsError::Validation(
                "approval currency must be provided".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Assigned,
    Approved,
    Declined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Approved,
    Declined,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision: ApprovalDecision,
    pub decided_by: UserId,
    pub decided_at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalTask {
    pub request: ApprovalRequest,
    pub status: ApprovalStatus,
    pub assigned_to: Option<UserId>,
    pub decision: Option<DecisionRecord>,
    pub current_stage_index: usize,
    pub stage_decisions: Vec<Option<DecisionRecord>>,
}

impl ApprovalTask {
    pub fn new(mut request: ApprovalRequest) -> Self {
        if request.stages.is_empty() {
            request.stages.push(ApprovalStage {
                approvers: Vec::new(),
            });
        }
        let stage_count = request.stages.len();
        Self {
            request,
            status: ApprovalStatus::Pending,
            assigned_to: None,
            decision: None,
            current_stage_index: 0,
            stage_decisions: vec![None; stage_count],
        }
    }

    pub fn is_finalized(&self) -> bool {
        matches!(
            self.status,
            ApprovalStatus::Approved | ApprovalStatus::Declined
        )
    }

    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        !self.is_finalized()
            && self
                .request
                .sla_at
                .map(|deadline| deadline < now)
                .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueueFilter {
    pub company_id: Option<CompanyId>,
    pub assignee: Option<UserId>,
    pub status: Option<ApprovalStatus>,
}

impl QueueFilter {
    pub fn matches(&self, task: &ApprovalTask) -> bool {
        if let Some(company_id) = &self.company_id
            && task.request.company_id != *company_id
        {
            return false;
        }
        if let Some(assignee) = &self.assignee
            && task.assigned_to.as_ref() != Some(assignee)
        {
            return false;
        }
        if let Some(status) = self.status
            && task.status != status
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionInput {
    pub decided_by: UserId,
    pub decision: ApprovalDecision,
    pub reason: Option<String>,
}

#[async_trait]
pub trait ApprovalsService: Send + Sync {
    async fn enqueue(&self, request: ApprovalRequest) -> ApprovalsResult<ApprovalTask>;
    async fn get(&self, approval_id: &ApprovalId) -> ApprovalsResult<ApprovalTask>;
    async fn list(&self, filter: QueueFilter) -> ApprovalsResult<Vec<ApprovalTask>>;
    async fn assign(
        &self,
        approval_id: &ApprovalId,
        assignee: UserId,
    ) -> ApprovalsResult<ApprovalTask>;
    async fn unassign(
        &self,
        approval_id: &ApprovalId,
        actor: &UserId,
    ) -> ApprovalsResult<ApprovalTask>;
    async fn decide(
        &self,
        approval_id: &ApprovalId,
        decision: DecisionInput,
    ) -> ApprovalsResult<ApprovalTask>;
    async fn overdue(&self, now: DateTime<Utc>) -> ApprovalsResult<Vec<ApprovalTask>>;
    async fn export_queue(&self) -> ApprovalsResult<QueueExport>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueExport {
    pub generated_at: DateTime<Utc>,
    pub tasks: Vec<ApprovalTask>,
}

#[derive(Default)]
pub struct InMemoryApprovalsService {
    tasks: RwLock<HashMap<ApprovalId, ApprovalTask>>,
}

impl InMemoryApprovalsService {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ApprovalsService for InMemoryApprovalsService {
    async fn enqueue(&self, request: ApprovalRequest) -> ApprovalsResult<ApprovalTask> {
        request.validate()?;
        let mut guard = self.tasks.write().await;
        let task = ApprovalTask::new(request);
        guard.insert(task.request.id.clone(), task.clone());
        Ok(task)
    }

    async fn get(&self, approval_id: &ApprovalId) -> ApprovalsResult<ApprovalTask> {
        let guard = self.tasks.read().await;
        guard
            .get(approval_id)
            .cloned()
            .ok_or_else(|| ApprovalsError::NotFound(approval_id.clone()))
    }

    async fn list(&self, filter: QueueFilter) -> ApprovalsResult<Vec<ApprovalTask>> {
        let guard = self.tasks.read().await;
        Ok(guard
            .values()
            .filter(|task| filter.matches(task))
            .cloned()
            .collect())
    }

    async fn assign(
        &self,
        approval_id: &ApprovalId,
        assignee: UserId,
    ) -> ApprovalsResult<ApprovalTask> {
        let mut guard = self.tasks.write().await;
        let task = guard
            .get_mut(approval_id)
            .ok_or_else(|| ApprovalsError::NotFound(approval_id.clone()))?;
        if task.is_finalized() {
            return Err(ApprovalsError::Finalized);
        }
        let stage = task
            .request
            .stages
            .get(task.current_stage_index)
            .ok_or_else(|| ApprovalsError::Validation("missing approval stage".into()))?;
        if !stage.allows(&assignee) {
            return Err(ApprovalsError::Validation(format!(
                "{assignee} is not an approver for stage {}",
                task.current_stage_index + 1
            )));
        }
        if let Some(current) = &task.assigned_to
            && current != &assignee
        {
            return Err(ApprovalsError::AlreadyAssigned {
                assignee: current.clone(),
            });
        }

        task.assigned_to = Some(assignee);
        task.status = ApprovalStatus::Assigned;
        Ok(task.clone())
    }

    async fn unassign(
        &self,
        approval_id: &ApprovalId,
        actor: &UserId,
    ) -> ApprovalsResult<ApprovalTask> {
        let mut guard = self.tasks.write().await;
        let task = guard
            .get_mut(approval_id)
            .ok_or_else(|| ApprovalsError::NotFound(approval_id.clone()))?;
        if task.is_finalized() {
            return Err(ApprovalsError::Finalized);
        }
        match &task.assigned_to {
            Some(current) if current == actor => {
                task.assigned_to = None;
                task.status = ApprovalStatus::Pending;
                Ok(task.clone())
            }
            Some(current) => Err(ApprovalsError::NotAssigned(current.clone())),
            None => Err(ApprovalsError::NotAssigned(actor.clone())),
        }
    }

    async fn decide(
        &self,
        approval_id: &ApprovalId,
        decision: DecisionInput,
    ) -> ApprovalsResult<ApprovalTask> {
        let mut guard = self.tasks.write().await;
        let task = guard
            .get_mut(approval_id)
            .ok_or_else(|| ApprovalsError::NotFound(approval_id.clone()))?;
        if task.is_finalized() {
            return Err(ApprovalsError::Finalized);
        }
        let stage = task
            .request
            .stages
            .get(task.current_stage_index)
            .ok_or_else(|| ApprovalsError::Validation("missing approval stage".into()))?;
        if !stage.allows(&decision.decided_by) {
            return Err(ApprovalsError::Validation(format!(
                "{} is not an approver for stage {}",
                decision.decided_by,
                task.current_stage_index + 1
            )));
        }
        if let Some(current) = &task.assigned_to
            && current != &decision.decided_by
        {
            return Err(ApprovalsError::NotAssigned(current.clone()));
        }

        let record = DecisionRecord {
            decision: decision.decision,
            decided_by: decision.decided_by,
            decided_at: Utc::now(),
            reason: decision.reason,
        };
        task.stage_decisions[task.current_stage_index] = Some(record.clone());
        task.assigned_to = None;

        match record.decision {
            ApprovalDecision::Approved => {
                if task.current_stage_index + 1 >= task.request.stages.len() {
                    task.status = ApprovalStatus::Approved;
                    task.decision = Some(record);
                } else {
                    task.current_stage_index += 1;
                    task.status = ApprovalStatus::Pending;
                    task.decision = None;
                }
            }
            ApprovalDecision::Declined => {
                task.status = ApprovalStatus::Declined;
                task.decision = Some(record);
            }
        }
        Ok(task.clone())
    }

    async fn overdue(&self, now: DateTime<Utc>) -> ApprovalsResult<Vec<ApprovalTask>> {
        let guard = self.tasks.read().await;
        let mut tasks = guard
            .values()
            .filter(|task| task.is_overdue(now))
            .cloned()
            .collect::<Vec<_>>();
        tasks.sort_by(|a, b| a.request.id.cmp(&b.request.id));
        Ok(tasks)
    }

    async fn export_queue(&self) -> ApprovalsResult<QueueExport> {
        let guard = self.tasks.read().await;
        let mut tasks = guard.values().cloned().collect::<Vec<_>>();
        tasks.sort_by(|a, b| a.request.id.cmp(&b.request.id));
        Ok(QueueExport {
            generated_at: Utc::now(),
            tasks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::Arc;

    fn make_request(company: &str, summary: &str) -> ApprovalRequest {
        let mut request = ApprovalRequest::new(company.into(), "user-1".into(), summary.into());
        request.amount_minor = 75_00;
        request.currency = "USD".into();
        request.stages = vec![ApprovalStage {
            approvers: vec!["approver-1".into(), "approver-2".into()],
        }];
        request
    }

    fn make_request_with_id(company: &str, summary: &str, id: &str) -> ApprovalRequest {
        let mut request = make_request(company, summary);
        request.id = id.into();
        request
    }

    #[tokio::test]
    async fn enqueue_and_assign_flow() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let task = service
            .enqueue(make_request("comp-1", "Post invoice INV-1001"))
            .await
            .expect("enqueue should succeed");

        assert_eq!(task.status, ApprovalStatus::Pending);
        assert_eq!(task.assigned_to, None);
        assert_eq!(task.current_stage_index, 0);
        assert_eq!(task.stage_decisions.len(), 1);
        assert!(task.stage_decisions[0].is_none());

        let assigned = service
            .assign(&task.request.id, "approver-1".into())
            .await
            .expect("assign should succeed");
        assert_eq!(assigned.status, ApprovalStatus::Assigned);
        assert_eq!(assigned.assigned_to, Some("approver-1".into()));
    }

    #[tokio::test]
    async fn prevent_double_assignment() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let task = service
            .enqueue(make_request("comp-1", "Large adjustment"))
            .await
            .expect("enqueue should succeed");

        let _ = service
            .assign(&task.request.id, "approver-1".into())
            .await
            .expect("first assign should succeed");

        let err = service
            .assign(&task.request.id, "approver-2".into())
            .await
            .expect_err("second assign should fail");
        match err {
            ApprovalsError::AlreadyAssigned { assignee } => {
                assert_eq!(assignee, "approver-1");
            }
            other => panic!("unexpected error {other:?}"),
        }
    }

    #[tokio::test]
    async fn decide_records_decision() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let task = service
            .enqueue(make_request("comp-1", "Post expense report"))
            .await
            .expect("enqueue should succeed");

        let _ = service
            .assign(&task.request.id, "approver-1".into())
            .await
            .expect("assignment should succeed");

        let decided = service
            .decide(
                &task.request.id,
                DecisionInput {
                    decided_by: "approver-1".into(),
                    decision: ApprovalDecision::Approved,
                    reason: Some("Matches policy threshold".into()),
                },
            )
            .await
            .expect("decision should succeed");

        assert_eq!(decided.status, ApprovalStatus::Approved);
        let record = decided.decision.expect("decision record required");
        assert_eq!(record.decision, ApprovalDecision::Approved);
        assert_eq!(record.decided_by, "approver-1");
        assert_eq!(record.reason, Some("Matches policy threshold".into()));
        assert_eq!(decided.stage_decisions[0].as_ref(), Some(&record));
    }

    #[tokio::test]
    async fn list_filters_by_company_and_status() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let first = service
            .enqueue(make_request("comp-1", "Review cash disbursement"))
            .await
            .expect("enqueue should succeed");
        let second = service
            .enqueue(make_request("comp-2", "Review credit memo"))
            .await
            .expect("enqueue should succeed");

        let _ = service
            .decide(
                &second.request.id,
                DecisionInput {
                    decided_by: "approver-2".into(),
                    decision: ApprovalDecision::Declined,
                    reason: None,
                },
            )
            .await
            .expect("decline should succeed");

        let pending = service
            .list(QueueFilter {
                company_id: Some("comp-1".into()),
                status: Some(ApprovalStatus::Pending),
                assignee: None,
            })
            .await
            .expect("list should succeed");

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request.id, first.request.id);
    }

    #[tokio::test]
    async fn multi_stage_requires_sequential_approval() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let mut request = make_request("comp-1", "Two stage approval");
        request.stages = vec![
            ApprovalStage {
                approvers: vec!["approver-1".into()],
            },
            ApprovalStage {
                approvers: vec!["approver-2".into()],
            },
        ];
        let task = service
            .enqueue(request)
            .await
            .expect("enqueue should succeed");

        let _ = service
            .assign(&task.request.id, "approver-1".into())
            .await
            .expect("assign first stage");
        let after_first = service
            .decide(
                &task.request.id,
                DecisionInput {
                    decided_by: "approver-1".into(),
                    decision: ApprovalDecision::Approved,
                    reason: None,
                },
            )
            .await
            .expect("first stage approval");
        assert_eq!(after_first.status, ApprovalStatus::Pending);
        assert_eq!(after_first.current_stage_index, 1);
        assert_eq!(
            after_first.stage_decisions[0]
                .as_ref()
                .map(|record| record.decision),
            Some(ApprovalDecision::Approved)
        );
        assert!(after_first.stage_decisions[1].is_none());

        let final_task = service
            .decide(
                &after_first.request.id,
                DecisionInput {
                    decided_by: "approver-2".into(),
                    decision: ApprovalDecision::Approved,
                    reason: None,
                },
            )
            .await
            .expect("second stage approval");
        assert_eq!(final_task.status, ApprovalStatus::Approved);
        assert_eq!(final_task.current_stage_index, 1);
        assert_eq!(
            final_task.stage_decisions[1]
                .as_ref()
                .map(|record| record.decision),
            Some(ApprovalDecision::Approved)
        );
    }

    #[tokio::test]
    async fn decline_short_circuits_remaining_stages() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let mut request = make_request("comp-1", "Decline early");
        request.stages = vec![
            ApprovalStage {
                approvers: vec!["approver-1".into()],
            },
            ApprovalStage {
                approvers: vec!["approver-2".into()],
            },
        ];
        let task = service.enqueue(request).await.expect("enqueue");
        let declined = service
            .decide(
                &task.request.id,
                DecisionInput {
                    decided_by: "approver-1".into(),
                    decision: ApprovalDecision::Declined,
                    reason: Some("Policy breach".into()),
                },
            )
            .await
            .expect("decline");
        assert_eq!(declined.status, ApprovalStatus::Declined);
        assert_eq!(declined.current_stage_index, 0);
        assert!(declined.stage_decisions[1].is_none());

        let follow_up = service
            .decide(
                &declined.request.id,
                DecisionInput {
                    decided_by: "approver-2".into(),
                    decision: ApprovalDecision::Approved,
                    reason: None,
                },
            )
            .await
            .expect_err("finalized tasks cannot transition");
        assert!(matches!(follow_up, ApprovalsError::Finalized));
    }

    #[tokio::test]
    async fn overdue_reports_tasks_past_sla() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let mut overdue_request = make_request("comp-1", "Urgent bill payment");
        overdue_request.sla_at = Some(Utc::now() - chrono::Duration::minutes(30));
        overdue_request.id = "overdue".into();
        let mut upcoming_request = make_request("comp-1", "Routine review");
        upcoming_request.sla_at = Some(Utc::now() + chrono::Duration::minutes(30));
        upcoming_request.id = "upcoming".into();

        service
            .enqueue(overdue_request)
            .await
            .expect("enqueue overdue");
        service
            .enqueue(upcoming_request)
            .await
            .expect("enqueue upcoming");

        let overdue = service
            .overdue(Utc::now())
            .await
            .expect("overdue query should succeed");
        assert_eq!(overdue.len(), 1);
        assert_eq!(overdue[0].request.id, "overdue");
    }

    #[tokio::test]
    async fn export_queue_serializes_current_state() {
        let service: Arc<dyn ApprovalsService> = Arc::new(InMemoryApprovalsService::new());
        let request_a = make_request_with_id("comp-1", "A", "task-a");
        let request_b = make_request_with_id("comp-1", "B", "task-b");
        service
            .enqueue(request_a)
            .await
            .expect("enqueue a succeeds");
        service
            .enqueue(request_b)
            .await
            .expect("enqueue b succeeds");

        let export = service.export_queue().await.expect("export should succeed");
        assert_eq!(export.tasks.len(), 2);
        assert!(export.generated_at <= Utc::now());
        assert_eq!(export.tasks[0].request.id, "task-a");
        assert_eq!(export.tasks[1].request.id, "task-b");
    }
}
