use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Context;
use async_trait::async_trait;
use codex_app_server_protocol::LedgerPeriodAction;
use codex_policy::PolicyDecision;
use codex_policy::PolicyEvaluationEvent;
use codex_policy::PolicyEventSink;
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryCounters {
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

#[derive(Debug)]
struct TelemetryStore {
    path: PathBuf,
}

impl TelemetryStore {
    fn from_env() -> Option<Self> {
        let home = env::var_os("CODEX_HOME")?;
        let mut path = PathBuf::from(home);
        path.push("accounting");
        path.push("telemetry.json");
        Some(Self { path })
    }

    fn read(&self) -> anyhow::Result<Option<TelemetryCounters>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let data = fs::read(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        let counters = serde_json::from_slice(&data)
            .with_context(|| format!("failed to parse {}", self.path.display()))?;
        Ok(Some(counters))
    }

    fn persist(&self, counters: &TelemetryCounters) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let data =
            serde_json::to_vec_pretty(counters).context("failed to encode telemetry counters")?;
        fs::write(&self.path, data)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct TelemetryInner {
    counters: TelemetryCounters,
    store: Option<TelemetryStore>,
}

impl TelemetryInner {
    fn with_store(store: Option<TelemetryStore>) -> Self {
        match store {
            Some(store) => {
                let counters = match store.read() {
                    Ok(Some(existing)) => existing,
                    Ok(None) => TelemetryCounters::default(),
                    Err(err) => {
                        warn!(
                            path = %store.path.display(),
                            error = %err,
                            "failed to load persisted telemetry; continuing with defaults"
                        );
                        TelemetryCounters::default()
                    }
                };
                Self {
                    counters,
                    store: Some(store),
                }
            }
            None => Self::default(),
        }
    }

    fn persist(&self) {
        if let Some(store) = &self.store
            && let Err(err) = store.persist(&self.counters)
        {
            warn!(
                path = %store.path.display(),
                error = %err,
                "failed to persist telemetry counters"
            );
        }
    }
}

#[derive(Clone, Default)]
pub struct AccountingTelemetry {
    inner: Arc<Mutex<TelemetryInner>>,
}

impl AccountingTelemetry {
    #[must_use]
    pub fn new() -> Self {
        Self::from_store(None)
    }

    #[must_use]
    pub fn persistent_from_env() -> Self {
        Self::from_store(TelemetryStore::from_env())
    }

    #[must_use]
    pub fn with_store_path(path: PathBuf) -> Self {
        Self::from_store(Some(TelemetryStore { path }))
    }

    fn from_store(store: Option<TelemetryStore>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TelemetryInner::with_store(store))),
        }
    }

    fn update<F>(&self, mut updater: F)
    where
        F: FnMut(&mut TelemetryCounters) -> bool,
    {
        if let Ok(mut inner) = self.inner.lock()
            && updater(&mut inner.counters)
        {
            inner.persist();
        }
    }

    pub fn record_transactions(&self, count: usize) {
        if count == 0 {
            return;
        }
        self.update(|counters| {
            counters.reconciliation_transactions += count;
            true
        });
    }

    pub fn record_candidates(&self, count: usize) {
        if count == 0 {
            return;
        }
        self.update(|counters| {
            counters.reconciliation_candidates += count;
            true
        });
    }

    pub fn record_write_off(&self) {
        self.update(|counters| {
            counters.reconciliation_write_offs += 1;
            true
        });
    }

    pub fn record_period_lock(&self, action: LedgerPeriodAction) {
        self.update(|counters| {
            counters.period_lock_events += 1;
            match action {
                LedgerPeriodAction::SoftClose => counters.period_lock_soft_close += 1,
                LedgerPeriodAction::Close => counters.period_lock_close += 1,
                LedgerPeriodAction::ReopenSoft => counters.period_lock_reopen_soft += 1,
                LedgerPeriodAction::ReopenFull => counters.period_lock_reopen_full += 1,
            }
            true
        });
    }

    pub fn record_policy_decision(&self, decision: PolicyDecision) {
        self.update(|counters| {
            match decision {
                PolicyDecision::AutoPost => counters.policy_auto_post += 1,
                PolicyDecision::NeedsApproval => counters.policy_needs_approval += 1,
                PolicyDecision::Reject => counters.policy_reject += 1,
            }
            true
        });
    }

    pub fn record_approvals_snapshot(&self, total: usize, overdue: usize) {
        self.update(|counters| {
            if counters.approvals_total == total && counters.approvals_overdue == overdue {
                return false;
            }
            counters.approvals_total = total;
            counters.approvals_overdue = overdue;
            true
        });
    }

    #[must_use]
    pub fn snapshot(&self) -> TelemetryCounters {
        self.inner
            .lock()
            .map(|inner| inner.counters.clone())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn policy_sink(&self) -> TelemetryPolicyEventSink {
        TelemetryPolicyEventSink {
            telemetry: self.clone(),
        }
    }

    #[must_use]
    pub fn store_path(&self) -> Option<PathBuf> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.store.as_ref().map(|store| store.path.clone()))
    }
}

#[derive(Clone)]
pub struct TelemetryPolicyEventSink {
    telemetry: AccountingTelemetry,
}

#[async_trait]
impl PolicyEventSink for TelemetryPolicyEventSink {
    async fn record(&self, event: PolicyEvaluationEvent) {
        self.telemetry.record_policy_decision(event.decision);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn counters_accumulate() {
        let telemetry = AccountingTelemetry::new();
        telemetry.record_transactions(3);
        telemetry.record_candidates(2);
        telemetry.record_write_off();
        telemetry.record_period_lock(LedgerPeriodAction::SoftClose);
        telemetry.record_period_lock(LedgerPeriodAction::Close);
        telemetry.record_approvals_snapshot(5, 2);
        telemetry.record_policy_decision(PolicyDecision::NeedsApproval);
        telemetry.record_policy_decision(PolicyDecision::AutoPost);
        let counters = telemetry.snapshot();
        assert_eq!(counters.reconciliation_transactions, 3);
        assert_eq!(counters.reconciliation_candidates, 2);
        assert_eq!(counters.reconciliation_write_offs, 1);
        assert_eq!(counters.period_lock_events, 2);
        assert_eq!(counters.policy_needs_approval, 1);
        assert_eq!(counters.policy_auto_post, 1);
        assert_eq!(counters.approvals_total, 5);
        assert_eq!(counters.approvals_overdue, 2);
    }

    #[test]
    fn persistence_survives_restart() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("telemetry.json");
        {
            let telemetry = AccountingTelemetry::with_store_path(path.clone());
            telemetry.record_transactions(4);
            telemetry.record_period_lock(LedgerPeriodAction::Close);
        }
        let telemetry = AccountingTelemetry::with_store_path(path);
        let counters = telemetry.snapshot();
        assert_eq!(counters.reconciliation_transactions, 4);
        assert_eq!(counters.period_lock_events, 1);
        assert_eq!(counters.period_lock_close, 1);
    }

    #[tokio::test]
    async fn policy_sink_records() {
        let telemetry = AccountingTelemetry::new();
        let sink = telemetry.policy_sink();
        let event = PolicyEvaluationEvent {
            company_id: "co-test".into(),
            proposal_id: "pp-1".into(),
            actor: "tester".into(),
            decision: PolicyDecision::Reject,
            triggers: Vec::new(),
            total_minor: 0,
            currency: "USD".into(),
            vendor_id: None,
            account_codes: Vec::new(),
            confidence: None,
            auto_post_limit_minor: 0,
            confidence_floor: None,
            evaluated_at: Utc::now(),
        };
        sink.record(event).await;
        let counters = telemetry.snapshot();
        assert_eq!(counters.policy_reject, 1);
    }

    #[test]
    fn persistence_recovers_from_corrupt_file() {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("telemetry.json");
        fs::write(&path, b"not json").expect("write corrupt telemetry");

        let telemetry = AccountingTelemetry::with_store_path(path.clone());
        let counters = telemetry.snapshot();
        assert_eq!(counters.reconciliation_transactions, 0);
        assert_eq!(counters.period_lock_events, 0);

        telemetry.record_transactions(2);
        telemetry.record_period_lock(LedgerPeriodAction::Close);

        let reloaded = AccountingTelemetry::with_store_path(path);
        let counters = reloaded.snapshot();
        assert_eq!(counters.reconciliation_transactions, 2);
        assert_eq!(counters.period_lock_events, 1);

        let stored_path = reloaded.store_path().expect("telemetry path recorded");
        assert!(
            stored_path.ends_with("telemetry.json"),
            "unexpected path: {stored_path:?}"
        );
    }
}
