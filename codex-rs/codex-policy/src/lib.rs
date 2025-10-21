#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type CompanyId = String;
pub type ProposalId = String;

pub type PolicyResult<T> = Result<T, PolicyError>;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyRuleSet {
    pub auto_post_enabled: bool,
    pub auto_post_limit_minor: i64,
    pub confidence_floor: Option<f32>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub approval_required_vendors: HashSet<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub approval_required_accounts: HashSet<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub blocked_vendors: HashSet<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub blocked_accounts: HashSet<String>,
}

impl Default for PolicyRuleSet {
    fn default() -> Self {
        Self {
            auto_post_enabled: false,
            auto_post_limit_minor: 50_000,
            confidence_floor: Some(0.8),
            approval_required_vendors: HashSet::new(),
            approval_required_accounts: HashSet::new(),
            blocked_vendors: HashSet::new(),
            blocked_accounts: HashSet::new(),
        }
    }
}

impl PolicyRuleSet {
    pub fn evaluate(&self, proposal: &PostingProposal) -> EvaluationOutcome {
        let mut approval = Vec::new();
        let mut rejects = Vec::new();

        if !self.auto_post_enabled {
            approval.push(PolicyTrigger::AutoPostDisabled);
        }

        if proposal.total_minor.abs() > self.auto_post_limit_minor {
            approval.push(PolicyTrigger::AmountExceedsLimit {
                limit_minor: self.auto_post_limit_minor,
                actual_minor: proposal.total_minor,
            });
        }

        if let Some(floor) = self.confidence_floor {
            match proposal.confidence {
                Some(observed) if observed + f32::EPSILON >= floor => {}
                Some(observed) => approval.push(PolicyTrigger::ConfidenceBelowFloor {
                    required: floor,
                    observed,
                }),
                None => approval.push(PolicyTrigger::ConfidenceMissing { required: floor }),
            }
        }

        if let Some(vendor) = &proposal.vendor_id {
            if self.blocked_vendors.contains(vendor) {
                rejects.push(PolicyTrigger::VendorBlocked {
                    vendor_id: vendor.clone(),
                });
            } else if self.approval_required_vendors.contains(vendor) {
                approval.push(PolicyTrigger::VendorRequiresApproval {
                    vendor_id: vendor.clone(),
                });
            }
        }

        for account in &proposal.account_codes {
            if self.blocked_accounts.contains(account) {
                rejects.push(PolicyTrigger::AccountBlocked {
                    account_code: account.clone(),
                });
            } else if self.approval_required_accounts.contains(account) {
                approval.push(PolicyTrigger::AccountRequiresApproval {
                    account_code: account.clone(),
                });
            }
        }

        let decision = if !rejects.is_empty() {
            PolicyDecision::Reject
        } else if !approval.is_empty() {
            PolicyDecision::NeedsApproval
        } else {
            PolicyDecision::AutoPost
        };

        let mut triggers = rejects;
        triggers.extend(approval);

        EvaluationOutcome { decision, triggers }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyRuleBinding {
    pub company_id: CompanyId,
    pub rules: PolicyRuleSet,
}

#[async_trait]
pub trait PolicyRulePersistence: Send + Sync {
    async fn write_rule_set(
        &self,
        company_id: &CompanyId,
        rules: &PolicyRuleSet,
    ) -> PolicyResult<()>;
    async fn read_rule_set(&self, company_id: &CompanyId) -> PolicyResult<Option<PolicyRuleSet>>;
    async fn read_all(&self) -> PolicyResult<Vec<PolicyRuleBinding>>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationOutcome {
    pub decision: PolicyDecision,
    pub triggers: Vec<PolicyTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    AutoPost,
    NeedsApproval,
    Reject,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PolicyTrigger {
    AutoPostDisabled,
    AmountExceedsLimit { limit_minor: i64, actual_minor: i64 },
    ConfidenceBelowFloor { required: f32, observed: f32 },
    ConfidenceMissing { required: f32 },
    VendorRequiresApproval { vendor_id: String },
    AccountRequiresApproval { account_code: String },
    VendorBlocked { vendor_id: String },
    AccountBlocked { account_code: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyEvaluationEvent {
    pub company_id: CompanyId,
    pub proposal_id: ProposalId,
    pub actor: String,
    pub decision: PolicyDecision,
    pub triggers: Vec<PolicyTrigger>,
    pub total_minor: i64,
    pub currency: String,
    pub vendor_id: Option<String>,
    pub account_codes: Vec<String>,
    pub confidence: Option<f32>,
    pub auto_post_limit_minor: i64,
    pub confidence_floor: Option<f32>,
    pub evaluated_at: DateTime<Utc>,
}

#[async_trait]
pub trait PolicyEventSink: Send + Sync {
    async fn record(&self, event: PolicyEvaluationEvent);
}

#[derive(Clone, Default)]
pub struct NoopPolicyEventSink;

#[async_trait]
impl PolicyEventSink for NoopPolicyEventSink {
    async fn record(&self, _event: PolicyEvaluationEvent) {}
}

#[derive(Default)]
pub struct InMemoryPolicyEventSink {
    events: RwLock<Vec<PolicyEvaluationEvent>>,
}

impl InMemoryPolicyEventSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn events(&self) -> Vec<PolicyEvaluationEvent> {
        let guard = self.events.read().await;
        guard.clone()
    }
}

#[async_trait]
impl PolicyEventSink for InMemoryPolicyEventSink {
    async fn record(&self, event: PolicyEvaluationEvent) {
        let mut guard = self.events.write().await;
        guard.push(event);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostingProposal {
    pub id: ProposalId,
    pub company_id: CompanyId,
    pub total_minor: i64,
    pub currency: String,
    pub vendor_id: Option<String>,
    pub account_codes: Vec<String>,
    pub confidence: Option<f32>,
    pub submitted_at: DateTime<Utc>,
}

impl PostingProposal {
    pub fn new(company_id: CompanyId, total_minor: i64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            company_id,
            total_minor,
            currency: "USD".into(),
            vendor_id: None,
            account_codes: Vec::new(),
            confidence: None,
            submitted_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyContext {
    pub company_id: CompanyId,
    pub actor: String,
}

#[async_trait]
pub trait PolicyStore: Send + Sync {
    async fn put_rule_set(&self, company_id: CompanyId, rules: PolicyRuleSet) -> PolicyResult<()>;
    async fn get_rule_set(&self, company_id: &CompanyId) -> PolicyResult<Option<PolicyRuleSet>>;
    async fn list_rule_sets(&self) -> PolicyResult<HashMap<CompanyId, PolicyRuleSet>>;
}

#[derive(Default)]
pub struct InMemoryPolicyStore {
    rules: RwLock<HashMap<CompanyId, PolicyRuleSet>>,
}

impl InMemoryPolicyStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl PolicyStore for InMemoryPolicyStore {
    async fn put_rule_set(&self, company_id: CompanyId, rules: PolicyRuleSet) -> PolicyResult<()> {
        let mut guard = self.rules.write().await;
        guard.insert(company_id, rules);
        Ok(())
    }

    async fn get_rule_set(&self, company_id: &CompanyId) -> PolicyResult<Option<PolicyRuleSet>> {
        let guard = self.rules.read().await;
        Ok(guard.get(company_id).cloned())
    }

    async fn list_rule_sets(&self) -> PolicyResult<HashMap<CompanyId, PolicyRuleSet>> {
        let guard = self.rules.read().await;
        Ok(guard.clone())
    }
}

#[async_trait]
impl PolicyRulePersistence for InMemoryPolicyStore {
    async fn write_rule_set(
        &self,
        company_id: &CompanyId,
        rules: &PolicyRuleSet,
    ) -> PolicyResult<()> {
        let mut guard = self.rules.write().await;
        guard.insert(company_id.clone(), rules.clone());
        Ok(())
    }

    async fn read_rule_set(&self, company_id: &CompanyId) -> PolicyResult<Option<PolicyRuleSet>> {
        let guard = self.rules.read().await;
        Ok(guard.get(company_id).cloned())
    }

    async fn read_all(&self) -> PolicyResult<Vec<PolicyRuleBinding>> {
        let guard = self.rules.read().await;
        Ok(guard
            .iter()
            .map(|(company_id, rules)| PolicyRuleBinding {
                company_id: company_id.clone(),
                rules: rules.clone(),
            })
            .collect())
    }
}

#[derive(Clone)]
pub struct DurablePolicyStore<P>
where
    P: PolicyRulePersistence + 'static,
{
    persistence: Arc<P>,
    cache: Arc<InMemoryPolicyStore>,
}

impl<P> DurablePolicyStore<P>
where
    P: PolicyRulePersistence + 'static,
{
    #[must_use]
    pub fn new(persistence: Arc<P>) -> Self {
        Self {
            persistence,
            cache: Arc::new(InMemoryPolicyStore::new()),
        }
    }

    async fn load_into_cache(&self, binding: &PolicyRuleBinding) -> PolicyResult<()> {
        PolicyStore::put_rule_set(
            &*self.cache,
            binding.company_id.clone(),
            binding.rules.clone(),
        )
        .await
    }
}

#[async_trait]
impl<P> PolicyStore for DurablePolicyStore<P>
where
    P: PolicyRulePersistence + 'static,
{
    async fn put_rule_set(&self, company_id: CompanyId, rules: PolicyRuleSet) -> PolicyResult<()> {
        self.persistence.write_rule_set(&company_id, &rules).await?;
        PolicyStore::put_rule_set(&*self.cache, company_id, rules).await
    }

    async fn get_rule_set(&self, company_id: &CompanyId) -> PolicyResult<Option<PolicyRuleSet>> {
        if let Some(cached) = PolicyStore::get_rule_set(&*self.cache, company_id).await? {
            return Ok(Some(cached));
        }
        let fetched = self.persistence.read_rule_set(company_id).await?;
        if let Some(ref rules) = fetched {
            PolicyStore::put_rule_set(&*self.cache, company_id.clone(), rules.clone()).await?;
        }
        Ok(fetched)
    }

    async fn list_rule_sets(&self) -> PolicyResult<HashMap<CompanyId, PolicyRuleSet>> {
        let bindings = self.persistence.read_all().await?;
        for binding in &bindings {
            self.load_into_cache(binding).await?;
        }
        let mut output = HashMap::new();
        for binding in bindings {
            output.insert(binding.company_id, binding.rules);
        }
        Ok(output)
    }
}

#[cfg(feature = "postgres-store")]
#[derive(Clone)]
pub struct PostgresPolicyStore {
    connection_string: String,
}

#[cfg(feature = "postgres-store")]
impl PostgresPolicyStore {
    /// Constructs a new Postgres-backed policy persistence stub.
    ///
    /// # Schema Draft
    /// ```sql
    /// CREATE TABLE policy_rule_sets (
    ///     company_id TEXT PRIMARY KEY,
    ///     rules JSONB NOT NULL,
    ///     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    /// );
    /// ```
    ///
    /// TODO: add optimistic locking via `updated_at` and persist policy history.
    #[must_use]
    pub fn new(connection_string: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
        }
    }
}

#[cfg(feature = "postgres-store")]
#[async_trait]
impl PolicyRulePersistence for PostgresPolicyStore {
    async fn write_rule_set(
        &self,
        company_id: &CompanyId,
        rules: &PolicyRuleSet,
    ) -> PolicyResult<()> {
        let _ = (&self.connection_string, company_id, rules);
        Err(PolicyError::Storage(
            "postgres store persistence not yet implemented".into(),
        ))
    }

    async fn read_rule_set(&self, company_id: &CompanyId) -> PolicyResult<Option<PolicyRuleSet>> {
        let _ = (&self.connection_string, company_id);
        Err(PolicyError::Storage(
            "postgres store persistence not yet implemented".into(),
        ))
    }

    async fn read_all(&self) -> PolicyResult<Vec<PolicyRuleBinding>> {
        let _ = &self.connection_string;
        Err(PolicyError::Storage(
            "postgres store persistence not yet implemented".into(),
        ))
    }
}

#[derive(Clone)]
pub struct PolicyEngine {
    store: Arc<dyn PolicyStore>,
    default_rules: PolicyRuleSet,
    event_sink: Arc<dyn PolicyEventSink>,
}

impl PolicyEngine {
    pub fn new(store: Arc<dyn PolicyStore>) -> Self {
        Self {
            store,
            default_rules: PolicyRuleSet::default(),
            event_sink: Arc::new(NoopPolicyEventSink),
        }
    }

    pub fn with_default(store: Arc<dyn PolicyStore>, default_rules: PolicyRuleSet) -> Self {
        Self {
            store,
            default_rules,
            event_sink: Arc::new(NoopPolicyEventSink),
        }
    }

    pub fn with_event_sink(
        store: Arc<dyn PolicyStore>,
        event_sink: Arc<dyn PolicyEventSink>,
    ) -> Self {
        Self {
            store,
            default_rules: PolicyRuleSet::default(),
            event_sink,
        }
    }

    pub fn with_components(
        store: Arc<dyn PolicyStore>,
        default_rules: PolicyRuleSet,
        event_sink: Arc<dyn PolicyEventSink>,
    ) -> Self {
        Self {
            store,
            default_rules,
            event_sink,
        }
    }

    pub async fn evaluate(
        &self,
        context: PolicyContext,
        proposal: PostingProposal,
    ) -> PolicyResult<EvaluationOutcome> {
        if context.company_id != proposal.company_id {
            return Err(PolicyError::Validation(
                "proposal company does not match policy context".into(),
            ));
        }

        if proposal.currency.trim().is_empty() {
            return Err(PolicyError::Validation(
                "proposal currency cannot be empty".into(),
            ));
        }

        let rules = match self.store.get_rule_set(&proposal.company_id).await? {
            Some(rules) => rules,
            None => self.default_rules.clone(),
        };

        let outcome = rules.evaluate(&proposal);
        let event = PolicyEvaluationEvent {
            company_id: proposal.company_id.clone(),
            proposal_id: proposal.id.clone(),
            actor: context.actor,
            decision: outcome.decision.clone(),
            triggers: outcome.triggers.clone(),
            total_minor: proposal.total_minor,
            currency: proposal.currency.clone(),
            vendor_id: proposal.vendor_id.clone(),
            account_codes: proposal.account_codes.clone(),
            confidence: proposal.confidence,
            auto_post_limit_minor: rules.auto_post_limit_minor,
            confidence_floor: rules.confidence_floor,
            evaluated_at: Utc::now(),
        };
        self.event_sink.record(event).await;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn make_rules() -> PolicyRuleSet {
        PolicyRuleSet {
            auto_post_enabled: true,
            auto_post_limit_minor: 100_000,
            confidence_floor: Some(0.75),
            approval_required_vendors: HashSet::new(),
            approval_required_accounts: HashSet::new(),
            blocked_vendors: HashSet::new(),
            blocked_accounts: HashSet::new(),
        }
    }

    fn base_proposal(total_minor: i64) -> PostingProposal {
        let mut proposal = PostingProposal::new("comp-1".into(), total_minor);
        proposal.currency = "USD".into();
        proposal.confidence = Some(0.9);
        proposal.account_codes = vec!["6000".into()];
        proposal
    }

    #[tokio::test]
    async fn evaluate_auto_posts_when_within_limits() {
        let store: Arc<dyn PolicyStore> = Arc::new(InMemoryPolicyStore::new());
        store
            .put_rule_set("comp-1".into(), make_rules())
            .await
            .expect("rules save");
        let engine = PolicyEngine::new(store);

        let outcome = engine
            .evaluate(
                PolicyContext {
                    company_id: "comp-1".into(),
                    actor: "user-1".into(),
                },
                base_proposal(20_000),
            )
            .await
            .expect("evaluation should succeed");

        assert_eq!(
            outcome,
            EvaluationOutcome {
                decision: PolicyDecision::AutoPost,
                triggers: Vec::new(),
            }
        );
    }

    #[tokio::test]
    async fn evaluate_requires_approval_when_over_limit() {
        let store: Arc<dyn PolicyStore> = Arc::new(InMemoryPolicyStore::new());
        store
            .put_rule_set("comp-1".into(), make_rules())
            .await
            .expect("rules save");
        let engine = PolicyEngine::new(store);

        let outcome = engine
            .evaluate(
                PolicyContext {
                    company_id: "comp-1".into(),
                    actor: "user-1".into(),
                },
                base_proposal(500_000),
            )
            .await
            .expect("evaluation should succeed");

        assert_eq!(
            outcome,
            EvaluationOutcome {
                decision: PolicyDecision::NeedsApproval,
                triggers: vec![PolicyTrigger::AmountExceedsLimit {
                    limit_minor: 100_000,
                    actual_minor: 500_000,
                }],
            }
        );
    }

    #[tokio::test]
    async fn evaluate_rejects_blocked_vendor() {
        let store: Arc<dyn PolicyStore> = Arc::new(InMemoryPolicyStore::new());
        let mut rules = make_rules();
        rules.blocked_vendors.insert("fraudulent-vendor".into());
        store
            .put_rule_set("comp-1".into(), rules)
            .await
            .expect("rules save");
        let engine = PolicyEngine::new(store);

        let mut proposal = base_proposal(10_000);
        proposal.vendor_id = Some("fraudulent-vendor".into());

        let outcome = engine
            .evaluate(
                PolicyContext {
                    company_id: "comp-1".into(),
                    actor: "user-1".into(),
                },
                proposal,
            )
            .await
            .expect("evaluation should succeed");

        assert_eq!(
            outcome,
            EvaluationOutcome {
                decision: PolicyDecision::Reject,
                triggers: vec![PolicyTrigger::VendorBlocked {
                    vendor_id: "fraudulent-vendor".into()
                }],
            }
        );
    }

    #[tokio::test]
    async fn evaluate_requires_approval_when_confidence_missing() {
        let store: Arc<dyn PolicyStore> = Arc::new(InMemoryPolicyStore::new());
        store
            .put_rule_set("comp-1".into(), make_rules())
            .await
            .expect("rules save");
        let engine = PolicyEngine::new(store);

        let mut proposal = base_proposal(10_000);
        proposal.confidence = None;

        let outcome = engine
            .evaluate(
                PolicyContext {
                    company_id: "comp-1".into(),
                    actor: "user-1".into(),
                },
                proposal,
            )
            .await
            .expect("evaluation should succeed");

        assert_eq!(
            outcome,
            EvaluationOutcome {
                decision: PolicyDecision::NeedsApproval,
                triggers: vec![PolicyTrigger::ConfidenceMissing { required: 0.75 }],
            }
        );
    }

    #[tokio::test]
    async fn evaluate_uses_default_rules_when_missing() {
        let store: Arc<dyn PolicyStore> = Arc::new(InMemoryPolicyStore::new());
        let mut proposal = base_proposal(10_000);
        proposal.confidence = Some(0.1);
        let engine = PolicyEngine::new(store.clone());

        let outcome = engine
            .evaluate(
                PolicyContext {
                    company_id: "comp-1".into(),
                    actor: "user-1".into(),
                },
                proposal,
            )
            .await
            .expect("evaluation should succeed");

        assert_eq!(
            outcome,
            EvaluationOutcome {
                decision: PolicyDecision::NeedsApproval,
                triggers: vec![
                    PolicyTrigger::AutoPostDisabled,
                    PolicyTrigger::ConfidenceBelowFloor {
                        required: 0.8,
                        observed: 0.1
                    }
                ],
            }
        );
    }

    #[tokio::test]
    async fn emits_evaluation_event() {
        let store: Arc<dyn PolicyStore> = Arc::new(InMemoryPolicyStore::new());
        store
            .put_rule_set("comp-1".into(), make_rules())
            .await
            .expect("rules save");
        let sink = Arc::new(InMemoryPolicyEventSink::new());
        let event_sink: Arc<dyn PolicyEventSink> = sink.clone();
        let engine = PolicyEngine::with_components(store, PolicyRuleSet::default(), event_sink);
        let mut proposal = base_proposal(42_000);
        proposal.id = "proposal-1".into();
        let outcome = engine
            .evaluate(
                PolicyContext {
                    company_id: "comp-1".into(),
                    actor: "user-1".into(),
                },
                proposal.clone(),
            )
            .await
            .expect("evaluation should succeed");
        assert_eq!(
            outcome,
            EvaluationOutcome {
                decision: PolicyDecision::AutoPost,
                triggers: Vec::new(),
            }
        );

        let events = sink.events().await;
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.company_id, "comp-1");
        assert_eq!(event.proposal_id, "proposal-1");
        assert_eq!(event.actor, "user-1");
        assert_eq!(event.decision, PolicyDecision::AutoPost);
        assert_eq!(event.triggers, Vec::new());
        assert_eq!(event.total_minor, 42_000);
        assert_eq!(event.currency, "USD");
        assert_eq!(event.vendor_id, None);
        assert_eq!(event.account_codes, vec![String::from("6000")]);
        assert_eq!(event.confidence, Some(0.9));
        assert_eq!(event.auto_post_limit_minor, 100_000);
        assert_eq!(event.confidence_floor, Some(0.75));
        assert!(event.evaluated_at >= proposal.submitted_at);
    }

    #[tokio::test]
    async fn durable_store_populates_cache_from_persistence() {
        let persistence = Arc::new(InMemoryPolicyStore::new());
        let rules = make_rules();
        PolicyStore::put_rule_set(&*persistence, "comp-1".into(), rules.clone())
            .await
            .expect("persist rules");

        let store: Arc<dyn PolicyStore> = Arc::new(DurablePolicyStore::new(persistence.clone()));

        let fetched = store
            .get_rule_set(&"comp-1".to_string())
            .await
            .expect("fetch rules")
            .expect("rules exist");
        assert_eq!(fetched, rules);

        let listed = store.list_rule_sets().await.expect("list rules");
        assert_eq!(listed.get("comp-1"), Some(&rules));
    }

    #[test]
    fn limit_and_confidence_matrix_matches_expectations() {
        struct Sample {
            auto_post_enabled: bool,
            limit: i64,
            floor: Option<f32>,
            amount: i64,
            confidence: Option<f32>,
            expected: EvaluationOutcome,
        }

        let samples = vec![
            Sample {
                auto_post_enabled: true,
                limit: 80_000,
                floor: Some(0.8),
                amount: 50_000,
                confidence: Some(0.85),
                expected: EvaluationOutcome {
                    decision: PolicyDecision::AutoPost,
                    triggers: Vec::new(),
                },
            },
            Sample {
                auto_post_enabled: true,
                limit: 80_000,
                floor: Some(0.9),
                amount: 50_000,
                confidence: Some(0.85),
                expected: EvaluationOutcome {
                    decision: PolicyDecision::NeedsApproval,
                    triggers: vec![PolicyTrigger::ConfidenceBelowFloor {
                        required: 0.9,
                        observed: 0.85,
                    }],
                },
            },
            Sample {
                auto_post_enabled: false,
                limit: 60_000,
                floor: Some(0.7),
                amount: 40_000,
                confidence: None,
                expected: EvaluationOutcome {
                    decision: PolicyDecision::NeedsApproval,
                    triggers: vec![
                        PolicyTrigger::AutoPostDisabled,
                        PolicyTrigger::ConfidenceMissing { required: 0.7 },
                    ],
                },
            },
            Sample {
                auto_post_enabled: true,
                limit: 30_000,
                floor: None,
                amount: 50_000,
                confidence: Some(0.95),
                expected: EvaluationOutcome {
                    decision: PolicyDecision::NeedsApproval,
                    triggers: vec![PolicyTrigger::AmountExceedsLimit {
                        limit_minor: 30_000,
                        actual_minor: 50_000,
                    }],
                },
            },
        ];

        for sample in samples {
            let Sample {
                auto_post_enabled,
                limit,
                floor,
                amount,
                confidence,
                expected,
            } = sample;
            let rules = PolicyRuleSet {
                auto_post_enabled,
                auto_post_limit_minor: limit,
                confidence_floor: floor,
                ..PolicyRuleSet::default()
            };

            let mut proposal = base_proposal(amount);
            proposal.confidence = confidence;
            let outcome = rules.evaluate(&proposal);
            assert_eq!(outcome, expected);
        }
    }
}
