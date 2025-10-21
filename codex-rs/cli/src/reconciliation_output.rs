use chrono::DateTime;
use chrono::Utc;
use codex_accounting_api::TelemetryCounters;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ReconciliationTelemetryOutput {
    pub reconciliation_transactions: usize,
    pub reconciliation_candidates: usize,
    pub reconciliation_write_offs: usize,
    pub period_lock_events: usize,
    pub period_lock_soft_close: usize,
    pub period_lock_close: usize,
    pub period_lock_reopen_soft: usize,
    pub period_lock_reopen_full: usize,
    pub policy_auto_post: usize,
    pub policy_needs_approval: usize,
    pub policy_reject: usize,
    pub approvals_total: usize,
    pub approvals_overdue: usize,
}

impl From<&TelemetryCounters> for ReconciliationTelemetryOutput {
    fn from(counters: &TelemetryCounters) -> Self {
        Self {
            reconciliation_transactions: counters.reconciliation_transactions,
            reconciliation_candidates: counters.reconciliation_candidates,
            reconciliation_write_offs: counters.reconciliation_write_offs,
            period_lock_events: counters.period_lock_events,
            period_lock_soft_close: counters.period_lock_soft_close,
            period_lock_close: counters.period_lock_close,
            period_lock_reopen_soft: counters.period_lock_reopen_soft,
            period_lock_reopen_full: counters.period_lock_reopen_full,
            policy_auto_post: counters.policy_auto_post,
            policy_needs_approval: counters.policy_needs_approval,
            policy_reject: counters.policy_reject,
            approvals_total: counters.approvals_total,
            approvals_overdue: counters.approvals_overdue,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IngestSnapshotOutput {
    pub ingested_total: usize,
    pub deduped_total: usize,
    pub duplicates_dropped: usize,
    pub last_feed_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApprovalsBacklogOutput {
    pub generated_at: String,
    pub total: usize,
    pub overdue: usize,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationStreamTickOutput {
    pub tick: usize,
    pub matched: usize,
    pub pending: usize,
    pub coverage_ratio: f32,
    pub coverage_percent: f32,
    pub approvals: ApprovalsBacklogOutput,
    pub ingest: IngestSnapshotOutput,
    pub telemetry: ReconciliationTelemetryOutput,
    pub telemetry_path: Option<String>,
    pub generated_at: DateTime<Utc>,
}
