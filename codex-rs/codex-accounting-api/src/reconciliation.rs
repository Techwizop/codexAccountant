use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use chrono::DateTime;
use chrono::Utc;
use codex_bank_ingest::NormalizedBankTransaction;
use codex_ledger::CompanyId;
use codex_reconcile::CandidateId;
use codex_reconcile::MatchCandidate;
use codex_reconcile::ReconciliationService;
use codex_reconcile::SessionId;

use crate::AccountingTelemetry;

/// Summary response for reconciliation dashboards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationSummary {
    pub company_id: CompanyId,
    pub matched: usize,
    pub pending: usize,
    pub last_refreshed_at: Option<DateTime<Utc>>,
}

impl ReconciliationSummary {
    #[must_use]
    pub fn coverage_ratio(&self) -> f32 {
        let total = self.matched + self.pending;
        if total == 0 {
            0.0
        } else {
            self.matched as f32 / total as f32
        }
    }
}

/// Trait abstraction so we can plug different reconciliation data sources.
pub trait ReconciliationSummaryProvider: Send + Sync {
    fn summary(&self, company_id: &CompanyId) -> anyhow::Result<ReconciliationSummary>;
}

/// Placeholder implementation until the reconciliation service is wired up.
pub struct NullReconciliationSummaryProvider;

impl ReconciliationSummaryProvider for NullReconciliationSummaryProvider {
    fn summary(&self, company_id: &CompanyId) -> anyhow::Result<ReconciliationSummary> {
        Ok(ReconciliationSummary {
            company_id: company_id.clone(),
            matched: 0,
            pending: 0,
            last_refreshed_at: None,
        })
    }
}

/// In-memory summary provider used by demos and tests.
#[derive(Default)]
pub struct InMemoryReconciliationSummaryProvider {
    summaries: RwLock<HashMap<CompanyId, ReconciliationSummary>>,
}

impl InMemoryReconciliationSummaryProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, summary: ReconciliationSummary) {
        if let Ok(mut guard) = self.summaries.write() {
            guard.insert(summary.company_id.clone(), summary);
        }
    }
}

impl ReconciliationSummaryProvider for InMemoryReconciliationSummaryProvider {
    fn summary(&self, company_id: &CompanyId) -> anyhow::Result<ReconciliationSummary> {
        let guard = self
            .summaries
            .read()
            .map_err(|_| anyhow::anyhow!("reconciliation summary store poisoned"))?;
        guard
            .get(company_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("summary for company {company_id} not found"))
    }
}

/// Abstracts the source of normalized bank transactions.
pub trait BankTransactionSource: Send + Sync {
    fn list_transactions(
        &self,
        company_id: &CompanyId,
    ) -> anyhow::Result<Vec<NormalizedBankTransaction>>;
}

#[derive(Default)]
pub struct InMemoryBankTransactionSource {
    transactions: RwLock<HashMap<CompanyId, Vec<NormalizedBankTransaction>>>,
}

impl InMemoryBankTransactionSource {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, company_id: &CompanyId, transactions: Vec<NormalizedBankTransaction>) {
        if let Ok(mut guard) = self.transactions.write() {
            guard.insert(company_id.clone(), transactions);
        }
    }
}

impl BankTransactionSource for InMemoryBankTransactionSource {
    fn list_transactions(
        &self,
        company_id: &CompanyId,
    ) -> anyhow::Result<Vec<NormalizedBankTransaction>> {
        let guard = self
            .transactions
            .read()
            .map_err(|_| anyhow::anyhow!("transaction source poisoned"))?;
        Ok(guard.get(company_id).cloned().unwrap_or_default())
    }
}

/// Facade wiring bank transaction ingest with reconciliation state.
#[derive(Clone)]
pub struct ReconciliationFacade {
    transactions: Arc<dyn BankTransactionSource>,
    service: Arc<dyn ReconciliationService>,
    summary: Arc<dyn ReconciliationSummaryProvider>,
    telemetry: Option<Arc<AccountingTelemetry>>,
}

impl ReconciliationFacade {
    pub fn new(
        transactions: Arc<dyn BankTransactionSource>,
        service: Arc<dyn ReconciliationService>,
    ) -> Self {
        Self::with_summary_and_telemetry(
            transactions,
            service,
            Arc::new(NullReconciliationSummaryProvider),
            None,
        )
    }

    pub fn with_summary(
        transactions: Arc<dyn BankTransactionSource>,
        service: Arc<dyn ReconciliationService>,
        summary: Arc<dyn ReconciliationSummaryProvider>,
    ) -> Self {
        Self::with_summary_and_telemetry(transactions, service, summary, None)
    }

    pub fn with_summary_and_telemetry(
        transactions: Arc<dyn BankTransactionSource>,
        service: Arc<dyn ReconciliationService>,
        summary: Arc<dyn ReconciliationSummaryProvider>,
        telemetry: Option<Arc<AccountingTelemetry>>,
    ) -> Self {
        Self {
            transactions,
            service,
            summary,
            telemetry,
        }
    }

    pub fn list_transactions(
        &self,
        company_id: &CompanyId,
    ) -> anyhow::Result<Vec<NormalizedBankTransaction>> {
        let transactions = self.transactions.list_transactions(company_id)?;
        if let Some(telemetry) = self.telemetry.as_ref() {
            telemetry.record_transactions(transactions.len());
        }
        Ok(transactions)
    }

    pub fn list_candidates(&self, session_id: &SessionId) -> anyhow::Result<Vec<MatchCandidate>> {
        let session = self
            .service
            .session(session_id)
            .map_err(|err| anyhow::anyhow!(err))?;
        if let Some(telemetry) = self.telemetry.as_ref() {
            telemetry.record_candidates(session.candidates.len());
        }
        Ok(session.candidates)
    }

    pub fn write_off_candidate(
        &self,
        session_id: &SessionId,
        candidate_id: &CandidateId,
        approval_reference: &str,
    ) -> anyhow::Result<MatchCandidate> {
        if approval_reference.trim().is_empty() {
            anyhow::bail!("approval reference must be provided for write-offs");
        }
        let candidate = self
            .service
            .write_off(session_id, candidate_id, approval_reference.to_string())
            .map_err(|err| anyhow::anyhow!(err))?;
        if let Some(telemetry) = self.telemetry.as_ref() {
            telemetry.record_write_off();
        }
        Ok(candidate)
    }

    pub fn summary(&self, company_id: &CompanyId) -> anyhow::Result<ReconciliationSummary> {
        self.summary.summary(company_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use codex_reconcile::CandidateStatus;
    use codex_reconcile::InMemoryReconciliationService;
    use codex_reconcile::LinearScoringStrategy;
    use codex_reconcile::MatchProposal;
    use pretty_assertions::assert_eq;
    use std::sync::Arc;

    use crate::AccountingTelemetry;

    #[test]
    fn null_provider_returns_zero_summary() {
        let provider = NullReconciliationSummaryProvider;
        let summary = provider
            .summary(&"co-1".to_string())
            .expect("summary should succeed");
        assert_eq!(summary.company_id, "co-1");
        assert_eq!(summary.matched, 0);
        assert_eq!(summary.pending, 0);
        assert!(summary.last_refreshed_at.is_none());
    }

    #[test]
    fn summary_provider_errors_when_missing() {
        let provider = InMemoryReconciliationSummaryProvider::new();
        let result = provider.summary(&"missing-co".to_string());
        assert!(result.is_err());
    }

    fn sample_transaction(id: &str, amount_minor: i64) -> NormalizedBankTransaction {
        NormalizedBankTransaction {
            transaction_id: id.into(),
            account_id: "acct-1".into(),
            posted_date: NaiveDate::from_ymd_opt(2024, 10, 18).expect("valid date"),
            amount_minor,
            currency: "USD".into(),
            description: "Sample transaction".into(),
            source_reference: Some(format!("REF-{id}")),
            source_checksum: Some(format!("CHK-{id}")),
            is_void: false,
            duplicate_metadata: Default::default(),
            currency_validation: Default::default(),
        }
    }

    fn reconciliation_facade() -> (
        ReconciliationFacade,
        Arc<InMemoryBankTransactionSource>,
        Arc<InMemoryReconciliationService>,
        Arc<InMemoryReconciliationSummaryProvider>,
    ) {
        let source = Arc::new(InMemoryBankTransactionSource::new());
        let service = Arc::new(InMemoryReconciliationService::new(Arc::new(
            LinearScoringStrategy::new(),
        )));
        let summary = Arc::new(InMemoryReconciliationSummaryProvider::new());
        let facade =
            ReconciliationFacade::with_summary(source.clone(), service.clone(), summary.clone());
        (facade, source, service, summary)
    }

    #[test]
    fn facade_lists_transactions() {
        let (facade, source, _, _) = reconciliation_facade();
        source.insert(&"co-1".into(), vec![sample_transaction("txn-1", 10_000)]);

        let transactions = facade
            .list_transactions(&"co-1".into())
            .expect("transactions should list");
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].transaction_id, "txn-1");
    }

    #[test]
    fn facade_lists_candidates_and_write_off() {
        let (facade, _, service, summaries) = reconciliation_facade();
        let session = service.create_session("co-2").expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                MatchProposal {
                    transaction_id: "txn-1".into(),
                    journal_entry_id: "je-1".into(),
                    amount_delta_minor: 0,
                    date_delta_days: 0,
                    transaction_description: "Utilities invoice".into(),
                    journal_description: "Utilities invoice".into(),
                    group_id: Some("grp-1".into()),
                },
            )
            .expect("candidate added");

        let candidates = facade
            .list_candidates(&session.id)
            .expect("candidates listed");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, candidate.id);

        let written_off = facade
            .write_off_candidate(&session.id, &candidate.id, "APR-99")
            .expect("write off");
        assert_eq!(written_off.status, CandidateStatus::WrittenOff);
        assert_eq!(written_off.write_off_reason.as_deref(), Some("APR-99"));

        summaries.insert(ReconciliationSummary {
            company_id: "co-2".into(),
            matched: 3,
            pending: 2,
            last_refreshed_at: None,
        });
        let summary = facade
            .summary(&"co-2".into())
            .expect("summary should resolve");
        assert_eq!(summary.matched, 3);
        assert_eq!(summary.pending, 2);
    }

    #[test]
    fn list_candidates_requires_existing_session() {
        let (facade, _, _, _) = reconciliation_facade();
        let result = facade.list_candidates(&"missing-session".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn write_off_requires_reference() {
        let (facade, _, service, _) = reconciliation_facade();
        let session = service
            .create_session("co-approval")
            .expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                MatchProposal {
                    transaction_id: "txn-approval".into(),
                    journal_entry_id: "je-approval".into(),
                    amount_delta_minor: 0,
                    date_delta_days: 0,
                    transaction_description: "Approval test".into(),
                    journal_description: "Approval test".into(),
                    group_id: None,
                },
            )
            .expect("candidate created");
        let result = facade.write_off_candidate(&session.id, &candidate.id, "  ");
        assert!(result.is_err());
    }

    #[test]
    fn telemetry_records_counts() {
        let source = Arc::new(InMemoryBankTransactionSource::new());
        let service = Arc::new(InMemoryReconciliationService::new(Arc::new(
            LinearScoringStrategy::new(),
        )));
        let summary = Arc::new(InMemoryReconciliationSummaryProvider::new());
        let telemetry = Arc::new(AccountingTelemetry::new());
        let facade = ReconciliationFacade::with_summary_and_telemetry(
            source.clone(),
            service.clone(),
            summary,
            Some(telemetry.clone()),
        );
        source.insert(
            &"co-telemetry".into(),
            vec![sample_transaction("txn-1", 1_000)],
        );
        let session = service
            .create_session("co-telemetry")
            .expect("session created");
        let candidate = service
            .add_candidate(
                &session.id,
                MatchProposal {
                    transaction_id: "txn-1".into(),
                    journal_entry_id: "je-1".into(),
                    amount_delta_minor: 0,
                    date_delta_days: 0,
                    transaction_description: "demo".into(),
                    journal_description: "demo".into(),
                    group_id: None,
                },
            )
            .expect("candidate added");

        let _ = facade
            .list_transactions(&"co-telemetry".into())
            .expect("transactions listed");
        let _ = facade
            .list_candidates(&session.id)
            .expect("candidates listed");
        let _ = facade
            .write_off_candidate(&session.id, &candidate.id, "APPROVAL")
            .expect("write off");

        let counters = telemetry.snapshot();
        assert_eq!(counters.reconciliation_transactions, 1);
        assert_eq!(counters.reconciliation_candidates, 1);
        assert_eq!(counters.reconciliation_write_offs, 1);
    }
}
