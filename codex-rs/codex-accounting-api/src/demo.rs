use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use codex_app_server_protocol::LedgerAccount;
use codex_app_server_protocol::LedgerAccountType;
use codex_app_server_protocol::LedgerCompany;
use codex_app_server_protocol::LedgerCreateCompanyParams;
use codex_app_server_protocol::LedgerCurrency;
use codex_app_server_protocol::LedgerCurrencyMode;
use codex_app_server_protocol::LedgerEntryOrigin;
use codex_app_server_protocol::LedgerEntryStatus;
use codex_app_server_protocol::LedgerFiscalCalendar;
use codex_app_server_protocol::LedgerJournalEntry;
use codex_app_server_protocol::LedgerJournalLine;
use codex_app_server_protocol::LedgerPostEntryParams;
use codex_app_server_protocol::LedgerPostingMode;
use codex_app_server_protocol::LedgerPostingSide;
use codex_app_server_protocol::LedgerReconciliationStatus;
use codex_app_server_protocol::LedgerUpsertAccountParams;
use codex_approvals::ApprovalPriority;
use codex_approvals::ApprovalRequest;
use codex_approvals::ApprovalStage;
use codex_approvals::ApprovalsService;
use codex_approvals::InMemoryApprovalsService;
use codex_approvals::QueueFilter;
use codex_bank_ingest::NormalizedBankTransaction;
use codex_bank_ingest::dedupe_transactions;
use codex_ledger::InMemoryLedgerService;
use codex_ledger::LedgerResult;
use codex_ledger::LedgerService;
use codex_ledger::Role as LedgerRole;
use codex_ledger::TenantContext as LedgerTenantContext;
use codex_policy::InMemoryPolicyStore;
use codex_policy::PolicyRuleSet;
use codex_policy::PolicyStore;
use codex_reconcile::InMemoryReconciliationService;
use codex_reconcile::LinearScoringStrategy;
use codex_reconcile::MatchProposal;
use codex_reconcile::ReconciliationService;
use codex_reconcile::SessionId;

use crate::AccountingTelemetry;
use crate::LedgerFacade;
use crate::ReconciliationFacade;
use crate::ReconciliationSummary;
use crate::controls::ApprovalsQueueView;
use crate::controls::ControlsFacade;
use crate::reconciliation::InMemoryBankTransactionSource;
use crate::reconciliation::InMemoryReconciliationSummaryProvider;

const DEMO_ADMIN_TENANT: &str = "ledger-admin";
const DEMO_USER_ID: &str = "codex-ledger-demo";

#[derive(Debug, Clone)]
pub struct DemoLedgerEntry {
    pub company_id: String,
    pub entry: LedgerJournalEntry,
}

#[derive(Debug, Clone)]
pub struct DemoLedgerData {
    pub companies: Vec<LedgerCompany>,
    pub accounts: Vec<LedgerAccount>,
    pub entries: Vec<DemoLedgerEntry>,
}

#[derive(Debug, Clone)]
pub struct DemoIngestSnapshot {
    pub ingested_total: usize,
    pub deduped_total: usize,
    pub duplicates_dropped: usize,
    pub last_ingest_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct DemoReconciliationContext {
    pub ledger: DemoLedgerData,
    pub company_id: String,
    pub session_id: SessionId,
    pub facade: ReconciliationFacade,
    pub approvals_facade: ControlsFacade,
    pub approvals_service: Arc<InMemoryApprovalsService>,
    pub approvals_view: ApprovalsQueueView,
    pub ingest_snapshot: DemoIngestSnapshot,
    pub transactions_source: Arc<InMemoryBankTransactionSource>,
    pub reconciliation_service: Arc<InMemoryReconciliationService>,
    pub summary_provider: Arc<InMemoryReconciliationSummaryProvider>,
    pub telemetry: Arc<AccountingTelemetry>,
}

pub fn demo_admin_tenant() -> LedgerTenantContext {
    LedgerTenantContext {
        tenant_id: DEMO_ADMIN_TENANT.to_string(),
        user_id: DEMO_USER_ID.to_string(),
        roles: vec![LedgerRole::ServiceAccount],
        locale: Some("en-US".to_string()),
    }
}

pub fn demo_company_tenant(company_id: &str) -> LedgerTenantContext {
    LedgerTenantContext {
        tenant_id: company_id.to_string(),
        user_id: DEMO_USER_ID.to_string(),
        roles: vec![LedgerRole::Accountant],
        locale: Some("en-US".to_string()),
    }
}

fn usd() -> LedgerCurrency {
    LedgerCurrency {
        code: "USD".to_string(),
        precision: 2,
    }
}

pub async fn seed_demo_ledger(facade: &LedgerFacade) -> LedgerResult<DemoLedgerData> {
    let admin = demo_admin_tenant();
    let company = facade
        .create_company(
            LedgerCreateCompanyParams {
                name: "Demo Manufacturing".to_string(),
                base_currency: usd(),
                fiscal_calendar: LedgerFiscalCalendar {
                    periods_per_year: 12,
                    opening_month: 1,
                },
            },
            admin.clone(),
        )
        .await?
        .company;

    let company_id = company.id.clone();

    let cash_account = facade
        .upsert_account(
            LedgerUpsertAccountParams {
                account: LedgerAccount {
                    id: "cash".to_string(),
                    company_id: company_id.clone(),
                    code: "1000".to_string(),
                    name: "Cash and Cash Equivalents".to_string(),
                    account_type: LedgerAccountType::Asset,
                    parent_account_id: None,
                    currency_mode: LedgerCurrencyMode::FunctionalOnly,
                    tax_code: None,
                    is_summary: false,
                    is_active: true,
                },
            },
            admin.clone(),
        )
        .await?
        .account;

    let revenue_account = facade
        .upsert_account(
            LedgerUpsertAccountParams {
                account: LedgerAccount {
                    id: "revenue".to_string(),
                    company_id: company_id.clone(),
                    code: "4000".to_string(),
                    name: "Product Revenue".to_string(),
                    account_type: LedgerAccountType::Revenue,
                    parent_account_id: None,
                    currency_mode: LedgerCurrencyMode::FunctionalOnly,
                    tax_code: None,
                    is_summary: false,
                    is_active: true,
                },
            },
            admin,
        )
        .await?
        .account;

    let usd = usd();
    let entry = LedgerJournalEntry {
        id: "je-demo-1".to_string(),
        journal_id: "jnl-gl".to_string(),
        status: LedgerEntryStatus::Draft,
        reconciliation_status: LedgerReconciliationStatus::Unreconciled,
        lines: vec![
            LedgerJournalLine {
                id: "je-demo-1-1".to_string(),
                account_id: cash_account.id.clone(),
                side: LedgerPostingSide::Debit,
                amount_minor: 12_500,
                currency: usd.clone(),
                functional_amount_minor: 12_500,
                functional_currency: usd.clone(),
                exchange_rate: None,
                tax_code: None,
                memo: Some("Cash sale receipt".to_string()),
            },
            LedgerJournalLine {
                id: "je-demo-1-2".to_string(),
                account_id: revenue_account.id.clone(),
                side: LedgerPostingSide::Credit,
                amount_minor: 12_500,
                currency: usd.clone(),
                functional_amount_minor: 12_500,
                functional_currency: usd,
                exchange_rate: None,
                tax_code: None,
                memo: Some("Recognize revenue".to_string()),
            },
        ],
        origin: LedgerEntryOrigin::Manual,
        memo: Some("Starter transaction posted by Codex CLI".to_string()),
        reverses_entry_id: None,
        reversed_by_entry_id: None,
    };

    let company_tenant = demo_company_tenant(&company_id);
    let posted = facade
        .post_entry(
            LedgerPostEntryParams {
                entry,
                mode: LedgerPostingMode::Commit,
            },
            company_tenant,
        )
        .await?
        .entry;

    Ok(DemoLedgerData {
        companies: vec![company],
        accounts: vec![cash_account, revenue_account],
        entries: vec![DemoLedgerEntry {
            company_id,
            entry: posted,
        }],
    })
}

pub async fn seed_demo_ledger_with_service(
    service: Arc<dyn LedgerService>,
) -> LedgerResult<DemoLedgerData> {
    let facade = LedgerFacade::new(service);
    seed_demo_ledger(&facade).await
}

pub async fn seed_demo_reconciliation() -> Result<DemoReconciliationContext> {
    let ledger_service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
    let ledger = seed_demo_ledger_with_service(ledger_service.clone())
        .await
        .map_err(|err| anyhow!(err))?;
    let company_id = ledger
        .companies
        .first()
        .map(|company| company.id.clone())
        .ok_or_else(|| anyhow!("demo ledger failed to seed a company"))?;

    let telemetry = Arc::new(AccountingTelemetry::persistent_from_env());
    let transactions_source = Arc::new(InMemoryBankTransactionSource::new());
    let today = Utc::now().date_naive();
    let build_tx =
        |id: &str, amount_minor: i64, days_ago: i64, description: &str, reference: &str| {
            NormalizedBankTransaction {
                transaction_id: id.to_string(),
                account_id: "operating-cash".into(),
                posted_date: today - Duration::days(days_ago),
                amount_minor,
                currency: "USD".into(),
                description: description.to_string(),
                source_reference: Some(reference.to_string()),
                source_checksum: None,
                is_void: false,
                duplicate_metadata: Default::default(),
                currency_validation: Default::default(),
            }
        };
    let raw_transactions = vec![
        build_tx("txn-001", 12_500, 1, "Stripe payout", "REF-001"),
        build_tx("txn-002", -4_200, 2, "Utility payment", "REF-002"),
        build_tx("txn-003", -8_100, 3, "Vendor ACH", "REF-003"),
        build_tx("txn-004", -1_500, 4, "Vendor ACH remainder", "REF-004"),
        build_tx("txn-005", 3_200, 5, "Card clearing deposit", "REF-005"),
        build_tx("txn-dup-002", -4_200, 2, "Utility payment", "REF-002"),
    ];
    let dedupe = dedupe_transactions(raw_transactions);
    transactions_source.insert(&company_id, dedupe.transactions.clone());
    let ingest_snapshot = DemoIngestSnapshot {
        ingested_total: dedupe.metrics.kept + dedupe.metrics.dropped,
        deduped_total: dedupe.metrics.kept,
        duplicates_dropped: dedupe.metrics.dropped,
        last_ingest_at: Utc::now() - Duration::hours(3),
    };

    let reconciliation_service = Arc::new(InMemoryReconciliationService::new(Arc::new(
        LinearScoringStrategy::new(),
    )));
    let summary_provider = Arc::new(InMemoryReconciliationSummaryProvider::new());
    let facade = ReconciliationFacade::with_summary_and_telemetry(
        transactions_source.clone(),
        reconciliation_service.clone(),
        summary_provider.clone(),
        Some(telemetry.clone()),
    );

    let session = reconciliation_service
        .create_session(&company_id)
        .map_err(|err| anyhow!(err))?;
    let session_id = session.id.clone();
    let journal_entry_id = ledger
        .entries
        .first()
        .map(|entry| entry.entry.id.clone())
        .unwrap_or_else(|| "je-demo-1".to_string());
    let proposal = |txn_id: &str,
                    amount_delta_minor: i64,
                    date_delta_days: i64,
                    txn_desc: &str,
                    journal_desc: &str,
                    group_id: Option<&str>| MatchProposal {
        transaction_id: txn_id.to_string(),
        journal_entry_id: journal_entry_id.clone(),
        amount_delta_minor,
        date_delta_days,
        transaction_description: txn_desc.to_string(),
        journal_description: journal_desc.to_string(),
        group_id: group_id.map(std::string::ToString::to_string),
    };

    let _primary = reconciliation_service
        .add_candidate(
            &session_id,
            proposal("txn-001", 0, 1, "Stripe payout", "Stripe settlement", None),
        )
        .map_err(|err| anyhow!(err))?;
    let split_a = reconciliation_service
        .add_candidate(
            &session_id,
            proposal(
                "txn-003",
                1_500,
                0,
                "Vendor ACH partial",
                "Split vendor invoice",
                Some("grp-split"),
            ),
        )
        .map_err(|err| anyhow!(err))?;
    let split_b = reconciliation_service
        .add_candidate(
            &session_id,
            proposal(
                "txn-004",
                -1_500,
                0,
                "Vendor ACH remainder",
                "Split vendor invoice",
                Some("grp-split"),
            ),
        )
        .map_err(|err| anyhow!(err))?;
    reconciliation_service
        .accept_partial(
            &session_id,
            "grp-split",
            vec![split_a.id.clone(), split_b.id.clone()],
        )
        .map_err(|err| anyhow!(err))?;
    let write_off = reconciliation_service
        .add_candidate(
            &session_id,
            proposal(
                "txn-002",
                -200,
                2,
                "Utility adjustment",
                "Previous month variance",
                Some("grp-utility"),
            ),
        )
        .map_err(|err| anyhow!(err))?;
    reconciliation_service
        .write_off(&session_id, &write_off.id, "APR-UTILITY-ADJ".to_string())
        .map_err(|err| anyhow!(err))?;
    let _pending = reconciliation_service
        .add_candidate(
            &session_id,
            proposal(
                "txn-005",
                3_200,
                5,
                "Card clearing deposit",
                "Card batch settlement",
                None,
            ),
        )
        .map_err(|err| anyhow!(err))?;

    summary_provider.insert(ReconciliationSummary {
        company_id: company_id.clone(),
        matched: 8,
        pending: 3,
        last_refreshed_at: Some(Utc::now() - Duration::minutes(12)),
    });

    let policy_store_state = Arc::new(InMemoryPolicyStore::new());
    policy_store_state
        .put_rule_set(company_id.clone(), PolicyRuleSet::default())
        .await
        .map_err(|err| anyhow!(err))?;
    let policy_store: Arc<dyn PolicyStore> = policy_store_state;
    let approvals_service = Arc::new(InMemoryApprovalsService::new());
    let mut overdue_request = ApprovalRequest::new(
        company_id.clone(),
        "ops-user".into(),
        "Write-off approval".into(),
    );
    overdue_request.amount_minor = 8_100;
    overdue_request.priority = ApprovalPriority::High;
    overdue_request.sla_at = Some(Utc::now() - Duration::hours(6));
    overdue_request.stages = vec![ApprovalStage {
        approvers: vec!["approver-finance".into()],
    }];
    approvals_service
        .enqueue(overdue_request)
        .await
        .map_err(|err| anyhow!(err))?;
    let mut upcoming_request = ApprovalRequest::new(
        company_id.clone(),
        "ops-user".into(),
        "Monthly reconciliation sign-off".into(),
    );
    upcoming_request.amount_minor = 5_000;
    upcoming_request.priority = ApprovalPriority::Normal;
    upcoming_request.sla_at = Some(Utc::now() + Duration::hours(4));
    upcoming_request.stages = vec![ApprovalStage {
        approvers: vec!["approver-controller".into()],
    }];
    approvals_service
        .enqueue(upcoming_request)
        .await
        .map_err(|err| anyhow!(err))?;

    let approvals_facade = ControlsFacade::with_telemetry(
        policy_store,
        approvals_service.clone(),
        Some(telemetry.clone()),
    );
    let approvals_view = approvals_facade
        .approvals_queue(QueueFilter {
            company_id: Some(company_id.clone()),
            ..QueueFilter::default()
        })
        .await
        .map_err(|err| anyhow!(err))?;

    Ok(DemoReconciliationContext {
        ledger,
        company_id,
        session_id,
        facade,
        approvals_facade,
        approvals_service,
        approvals_view,
        ingest_snapshot,
        transactions_source,
        reconciliation_service,
        summary_provider,
        telemetry,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_ledger::InMemoryLedgerService;
    use codex_ledger::LedgerError;
    use pretty_assertions::assert_eq;
    use tokio::runtime::Runtime;

    #[test]
    fn demo_admin_and_company_tenants_have_expected_roles() {
        let admin = demo_admin_tenant();
        assert_eq!(admin.roles, vec![LedgerRole::ServiceAccount]);
        let company = demo_company_tenant("co-1");
        assert_eq!(company.roles, vec![LedgerRole::Accountant]);
        assert_eq!(company.tenant_id, "co-1");
    }

    #[test]
    fn seeding_demo_ledger_returns_company_accounts_and_entry() {
        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
            let data = seed_demo_ledger_with_service(service)
                .await
                .expect("demo ledger");

            assert_eq!(data.companies.len(), 1);
            assert_eq!(data.accounts.len(), 2);
            assert_eq!(data.entries.len(), 1);
            assert_eq!(data.entries[0].entry.lines.len(), 2);
        });
    }

    #[test]
    fn seeding_demo_ledger_with_invalid_service_surfaces_error() {
        struct FailingService;

        #[async_trait::async_trait]
        impl LedgerService for FailingService {
            async fn create_company(
                &self,
                _request: codex_ledger::CreateCompanyRequest,
            ) -> LedgerResult<codex_ledger::Company> {
                Err(LedgerError::Internal("boom".to_string()))
            }

            async fn upsert_account(
                &self,
                _request: codex_ledger::UpsertAccountRequest,
            ) -> LedgerResult<codex_ledger::Account> {
                unreachable!("not called")
            }

            async fn seed_chart(
                &self,
                _request: codex_ledger::SeedChartRequest,
            ) -> LedgerResult<Vec<codex_ledger::Account>> {
                unreachable!("not called")
            }

            async fn post_entry(
                &self,
                _request: codex_ledger::PostEntryRequest,
            ) -> LedgerResult<codex_ledger::JournalEntry> {
                unreachable!("not called")
            }

            async fn reverse_entry(
                &self,
                _request: codex_ledger::ReverseEntryRequest,
            ) -> LedgerResult<codex_ledger::JournalEntry> {
                unreachable!("not called")
            }

            async fn lock_period(
                &self,
                _request: codex_ledger::LockPeriodRequest,
            ) -> LedgerResult<codex_ledger::Journal> {
                unreachable!("not called")
            }

            async fn ensure_period(
                &self,
                _request: codex_ledger::EnsurePeriodRequest,
            ) -> LedgerResult<codex_ledger::Journal> {
                unreachable!("not called")
            }

            async fn revalue_currency(
                &self,
                _request: codex_ledger::CurrencyRevaluationRequest,
            ) -> LedgerResult<Vec<codex_ledger::JournalEntry>> {
                unreachable!("not called")
            }

            async fn list_audit_trail(
                &self,
                _filter: codex_ledger::AuditTrailFilter,
            ) -> LedgerResult<Vec<codex_ledger::AuditEvent>> {
                unreachable!("not called")
            }
        }

        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let service: Arc<dyn LedgerService> = Arc::new(FailingService);
            let result = seed_demo_ledger_with_service(service).await;
            assert!(matches!(result, Err(LedgerError::Internal(msg)) if msg == "boom"));
        });
    }

    #[tokio::test]
    async fn seed_demo_reconciliation_builds_context() {
        let context = seed_demo_reconciliation()
            .await
            .expect("reconciliation context");
        let transactions = context
            .facade
            .list_transactions(&context.company_id)
            .expect("transactions listed");
        assert!(!transactions.is_empty());
        let candidates = context
            .facade
            .list_candidates(&context.session_id)
            .expect("candidates listed");
        assert!(!candidates.is_empty());
        let summary = context
            .facade
            .summary(&context.company_id)
            .expect("summary listed");
        assert!(summary.matched > 0);
        assert!(context.ingest_snapshot.ingested_total >= context.ingest_snapshot.deduped_total);
        assert!(!context.approvals_view.tasks.is_empty());
        let counters = context.telemetry.snapshot();
        assert!(counters.reconciliation_candidates >= candidates.len());
    }
}
