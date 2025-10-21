//! Contract-style tests describing the expected async `LedgerService` MVP.
//!
//! These tests intentionally **fail** today because the service surface is not
//! implemented. They document the behaviour required by the accounting
//! architecture (docs/accounting/architecture.md): multi-company bootstrap,
//! chart-of-accounts (COA) validation, double-entry enforcement, FX provenance,
//! period controls, and reversible postings.

use std::time::SystemTime;

use codex_ledger::InMemoryLedgerService;
use codex_ledger::*;

fn usd() -> Currency {
    Currency {
        code: "USD".into(),
        precision: 2,
    }
}

fn eur() -> Currency {
    Currency {
        code: "EUR".into(),
        precision: 2,
    }
}

fn tenant_context() -> TenantContext {
    TenantContext {
        tenant_id: "tenant-1".into(),
        user_id: "user-1".into(),
        roles: vec![Role::Accountant],
        locale: Some("en-US".into()),
    }
}

fn tenant_context_for_company(company_id: &CompanyId, user_id: &str) -> TenantContext {
    TenantContext {
        tenant_id: company_id.clone(),
        user_id: user_id.into(),
        roles: vec![Role::Accountant],
        locale: Some("en-US".into()),
    }
}

fn company_request() -> CreateCompanyRequest {
    CreateCompanyRequest {
        name: "Test Co".into(),
        base_currency: usd(),
        fiscal_calendar: FiscalCalendar {
            periods_per_year: 12,
            opening_month: 1,
        },
        tenant: tenant_context(),
    }
}

fn asset_summary(company_id: &CompanyId) -> Account {
    Account {
        id: "acc-assets".into(),
        company_id: company_id.clone(),
        code: "1000".into(),
        name: "Assets".into(),
        account_type: AccountType::Asset,
        parent_account_id: None,
        currency_mode: CurrencyMode::FunctionalOnly,
        tax_code: None,
        is_summary: true,
        is_active: true,
    }
}

fn cash_account(company_id: &CompanyId, parent_id: AccountId) -> Account {
    Account {
        id: "acc-cash".into(),
        company_id: company_id.clone(),
        code: "1010".into(),
        name: "Cash".into(),
        account_type: AccountType::Asset,
        parent_account_id: Some(parent_id),
        currency_mode: CurrencyMode::FunctionalOnly,
        tax_code: None,
        is_summary: false,
        is_active: true,
    }
}

fn revenue_account(company_id: &CompanyId) -> Account {
    Account {
        id: "acc-rev".into(),
        company_id: company_id.clone(),
        code: "4000".into(),
        name: "Revenue".into(),
        account_type: AccountType::Revenue,
        parent_account_id: None,
        currency_mode: CurrencyMode::FunctionalOnly,
        tax_code: None,
        is_summary: false,
        is_active: true,
    }
}

fn build_post_entry(
    tenant: TenantContext,
    entry_id: &str,
    debit_account: &str,
    credit_account: &str,
    amount_minor: i64,
    memo: &str,
) -> PostEntryRequest {
    PostEntryRequest {
        entry: JournalEntry {
            id: entry_id.into(),
            journal_id: "jnl-gl".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            lines: vec![
                JournalLine {
                    id: format!("{entry_id}-debit"),
                    account_id: debit_account.into(),
                    side: PostingSide::Debit,
                    amount_minor,
                    currency: usd(),
                    functional_amount_minor: amount_minor,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some(memo.into()),
                },
                JournalLine {
                    id: format!("{entry_id}-credit"),
                    account_id: credit_account.into(),
                    side: PostingSide::Credit,
                    amount_minor,
                    currency: usd(),
                    functional_amount_minor: amount_minor,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some(memo.into()),
                },
            ],
            origin: EntryOrigin::Manual,
            memo: Some(memo.into()),
            reverses_entry_id: None,
            reversed_by_entry_id: None,
        },
        tenant,
        mode: PostingMode::Commit,
    }
}

fn journal_entry(tenant: TenantContext) -> PostEntryRequest {
    PostEntryRequest {
        entry: JournalEntry {
            id: "je-1".into(),
            journal_id: "jnl-gl".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "acc-cash".into(),
                    side: PostingSide::Debit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some("Invoice payment".into()),
                },
                JournalLine {
                    id: "ln-2".into(),
                    account_id: "acc-rev".into(),
                    side: PostingSide::Credit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some("Invoice payment".into()),
                },
            ],
            origin: EntryOrigin::Manual,
            memo: Some("Payment received".into()),
            reverses_entry_id: None,
            reversed_by_entry_id: None,
        },
        tenant,
        mode: PostingMode::Commit,
    }
}

fn unbalanced_entry(tenant: TenantContext) -> PostEntryRequest {
    PostEntryRequest {
        entry: JournalEntry {
            id: "je-imbalanced".into(),
            journal_id: "jnl-gl".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "acc-cash".into(),
                    side: PostingSide::Debit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: None,
                },
                JournalLine {
                    id: "ln-2".into(),
                    account_id: "acc-rev".into(),
                    side: PostingSide::Credit,
                    amount_minor: 9_000,
                    currency: usd(),
                    functional_amount_minor: 9_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: None,
                },
            ],
            origin: EntryOrigin::Manual,
            memo: Some("Out-of-balance".into()),
            reverses_entry_id: None,
            reversed_by_entry_id: None,
        },
        tenant,
        mode: PostingMode::Commit,
    }
}

fn fx_entry(tenant: TenantContext) -> PostEntryRequest {
    PostEntryRequest {
        entry: JournalEntry {
            id: "je-fx".into(),
            journal_id: "jnl-gl".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "acc-cash".into(),
                    side: PostingSide::Debit,
                    amount_minor: 9_300,
                    currency: eur(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: Some(CurrencyRate {
                        base: eur(),
                        quote: usd(),
                        rate: 1.075,
                        source: Some("ECB".into()),
                        observed_at: SystemTime::now(),
                    }),
                    tax_code: None,
                    memo: Some("FX receipt".into()),
                },
                JournalLine {
                    id: "ln-2".into(),
                    account_id: "acc-rev".into(),
                    side: PostingSide::Credit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some("FX receipt".into()),
                },
            ],
            origin: EntryOrigin::Manual,
            memo: Some("FX payment".into()),
            reverses_entry_id: None,
            reversed_by_entry_id: None,
        },
        tenant,
        mode: PostingMode::Commit,
    }
}

fn fx_entry_missing_rate(tenant: TenantContext) -> PostEntryRequest {
    PostEntryRequest {
        entry: JournalEntry {
            id: "je-fx-missing".into(),
            journal_id: "jnl-gl".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "acc-cash".into(),
                    side: PostingSide::Debit,
                    amount_minor: 9_300,
                    currency: eur(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some("Missing FX".into()),
                },
                JournalLine {
                    id: "ln-2".into(),
                    account_id: "acc-rev".into(),
                    side: PostingSide::Credit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: Some("Missing FX".into()),
                },
            ],
            origin: EntryOrigin::Manual,
            memo: Some("FX without provenance".into()),
            reverses_entry_id: None,
            reversed_by_entry_id: None,
        },
        tenant,
        mode: PostingMode::Commit,
    }
}

fn lock_request(company_id: &CompanyId, action: PeriodAction) -> LockPeriodRequest {
    LockPeriodRequest {
        journal_id: "jnl-gl".into(),
        period: PeriodRef {
            fiscal_year: 2025,
            period: 12,
        },
        action,
        approval_reference: None,
        tenant: tenant_context_for_company(company_id, "user-1"),
    }
}

#[tokio::test]
async fn company_bootstrap_and_coa_constraints() {
    let service = InMemoryLedgerService::new();

    let company = service
        .create_company(company_request())
        .await
        .expect("create_company should provision tenant-scoped company");
    assert_eq!(company.base_currency.code, "USD");

    // COA root summary account should be allowed, but not used for postings.
    let tenant = tenant_context_for_company(&company.id, "user-1");
    let summary = asset_summary(&company.id);
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: summary.clone(),
            tenant: tenant.clone(),
        })
        .await
        .expect("summary account is allowed for hierarchy");

    let posting_account = cash_account(&company.id, summary.id.clone());
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: posting_account.clone(),
            tenant: tenant.clone(),
        })
        .await
        .expect("leaf posting account should be accepted");

    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: revenue_account(&company.id),
            tenant: tenant.clone(),
        })
        .await
        .expect("revenue account should be provisioned in base currency");

    // Duplicate codes or summary postings should be rejected with clear validation errors.
    let duplicate_err = service
        .upsert_account(UpsertAccountRequest {
            account: posting_account.clone(),
            tenant: tenant.clone(),
        })
        .await
        .expect_err("duplicate account code must raise validation error");
    assert!(matches!(
        duplicate_err,
        LedgerError::Validation(message) if message.contains("code")
    ));

    let mut invalid_child = cash_account(&company.id, posting_account.id.clone());
    invalid_child.id = "acc-invalid".into();
    invalid_child.code = "1011".into();
    let invalid_parent_err = service
        .upsert_account(UpsertAccountRequest {
            account: invalid_child,
            tenant,
        })
        .await
        .expect_err("non-summary parent should be rejected");
    assert!(matches!(
        invalid_parent_err,
        LedgerError::Validation(message) if message.contains("parent")
    ));
}

#[tokio::test]
async fn posting_requires_balance_and_fx_provenance() {
    let service = InMemoryLedgerService::new();
    let company = service
        .create_company(company_request())
        .await
        .expect("company should be created");
    let tenant = tenant_context_for_company(&company.id, "user-1");

    let summary = asset_summary(&company.id);
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: summary.clone(),
            tenant: tenant.clone(),
        })
        .await
        .expect("summary account");

    let cash = cash_account(&company.id, summary.id.clone());
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: cash,
            tenant: tenant.clone(),
        })
        .await
        .expect("cash account");

    let revenue = revenue_account(&company.id);
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: revenue,
            tenant: tenant.clone(),
        })
        .await
        .expect("revenue account");

    // Balanced entry should be committed successfully.
    let posted = service
        .post_entry(journal_entry(tenant.clone()))
        .await
        .expect("balanced entry should post");
    assert_eq!(posted.status, EntryStatus::Posted);

    // Unbalanced entry must be rejected.
    let imbalance = service
        .post_entry(unbalanced_entry(tenant.clone()))
        .await
        .expect_err("debits and credits must balance");
    assert!(matches!(imbalance, LedgerError::Validation(message) if message.contains("balance")));

    // FX postings require provenance metadata.
    let fx_posted = service
        .post_entry(fx_entry(tenant.clone()))
        .await
        .expect("FX entry with rate metadata should post");
    assert!(
        fx_posted
            .lines
            .iter()
            .all(JournalLine::has_currency_provenance)
    );
    let retained_rate = fx_posted
        .lines
        .iter()
        .find_map(|line| line.exchange_rate.as_ref())
        .expect("FX line should retain exchange rate");
    assert!(
        (retained_rate.rate - 1.075).abs() < f64::EPSILON,
        "expected rate to remain unchanged"
    );

    let missing_fx = service
        .post_entry(fx_entry_missing_rate(tenant))
        .await
        .expect_err("FX entry without rate should fail validation");
    assert!(matches!(
        missing_fx,
        LedgerError::Validation(message) if message.contains("Currency amounts")
    ));
}

#[tokio::test]
async fn period_lock_and_reversal_flow() {
    let service = InMemoryLedgerService::new();
    let company = service
        .create_company(company_request())
        .await
        .expect("company should be created");
    let tenant = tenant_context_for_company(&company.id, "user-1");

    let summary = asset_summary(&company.id);
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: summary.clone(),
            tenant: tenant.clone(),
        })
        .await
        .expect("summary account");

    let cash = cash_account(&company.id, summary.id.clone());
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: cash,
            tenant: tenant.clone(),
        })
        .await
        .expect("cash account");

    let revenue = revenue_account(&company.id);
    let _ = service
        .upsert_account(UpsertAccountRequest {
            account: revenue,
            tenant: tenant.clone(),
        })
        .await
        .expect("revenue account");

    // Lock the period (soft close) and ensure state transitions.
    let locked = service
        .lock_period(lock_request(&company.id, PeriodAction::SoftClose))
        .await
        .expect("soft-close should succeed with appropriate role");
    assert_eq!(locked.period_state, PeriodState::SoftClosed);

    // Attempting to post while soft-closed without override should be rejected.
    let blocked = service
        .post_entry(journal_entry(tenant.clone()))
        .await
        .expect_err("posting into soft-closed period requires override");
    assert!(matches!(blocked, LedgerError::Rejected(message) if message.contains("soft-close")));

    // Fully closing period should block postings until reopened.
    let closed = service
        .lock_period(lock_request(&company.id, PeriodAction::Close))
        .await
        .expect("close should transition to Closed state");
    assert_eq!(closed.period_state, PeriodState::Closed);

    let closed_block = service
        .post_entry(journal_entry(tenant.clone()))
        .await
        .expect_err("posting into closed period must be rejected");
    assert!(matches!(
        closed_block,
        LedgerError::Rejected(message) if message.contains("period closed")
    ));

    service
        .lock_period(lock_request(&company.id, PeriodAction::ReopenFull))
        .await
        .expect("reopen should set state back to Open");

    let posted = service
        .post_entry(journal_entry(tenant.clone()))
        .await
        .expect("entry should post after reopening");
    assert_eq!(posted.status, EntryStatus::Posted);
    assert_eq!(posted.reverses_entry_id, None);
    assert_eq!(posted.reversed_by_entry_id, None);

    let reversal = service
        .reverse_entry(ReverseEntryRequest {
            entry_id: posted.id.clone(),
            reason: "User requested reversal".into(),
            tenant,
        })
        .await
        .expect("reverse_entry should create linked reversing journal");
    assert_eq!(reversal.status, EntryStatus::Posted);
    assert_eq!(
        reversal.reverses_entry_id.as_deref(),
        Some(posted.id.as_str())
    );
    assert!(reversal.reversed_by_entry_id.is_none());
    assert!(reversal.validate().is_ok());

    let double_reversal = service
        .reverse_entry(ReverseEntryRequest {
            entry_id: posted.id.clone(),
            reason: "duplicate".into(),
            tenant: tenant_context_for_company(&company.id, "user-1"),
        })
        .await
        .expect_err("second reversal must be rejected");
    assert!(matches!(
        double_reversal,
        LedgerError::Rejected(message) if message.contains("already reversed")
    ));

    let original_events = service
        .list_audit_trail(AuditTrailFilter {
            entity_id: Some(posted.id.clone()),
            limit: None,
            cursor: None,
            tenant: tenant_context_for_company(&company.id, "auditor"),
        })
        .await
        .expect("audit events should be returned");
    assert!(
        original_events
            .iter()
            .any(|event| event.description.contains("Posted entry"))
    );
    assert!(
        original_events
            .iter()
            .any(|event| event.description.contains("Reversal requested"))
    );

    let reversal_events = service
        .list_audit_trail(AuditTrailFilter {
            entity_id: Some(reversal.id.clone()),
            limit: None,
            cursor: None,
            tenant: tenant_context_for_company(&company.id, "auditor"),
        })
        .await
        .expect("reversal entry should have audit trail");
    assert!(
        reversal_events
            .iter()
            .any(|event| event.description.contains("Reversal of"))
    );
}

#[tokio::test]
async fn reverse_missing_entry_returns_not_found() {
    let service = InMemoryLedgerService::new();
    let company = service
        .create_company(company_request())
        .await
        .expect("company should be created");

    let err = service
        .reverse_entry(ReverseEntryRequest {
            entry_id: "je-missing".into(),
            reason: "not present".into(),
            tenant: tenant_context_for_company(&company.id, "user-1"),
        })
        .await
        .expect_err("missing entry must return NotFound");

    assert!(matches!(
        err,
        LedgerError::NotFound(message) if message.contains("je-missing")
    ));
}

#[tokio::test]
async fn multi_company_journal_isolation() {
    let service = InMemoryLedgerService::new();

    let company_a = service
        .create_company(company_request())
        .await
        .expect("company A should be created");
    let company_b = service
        .create_company(company_request())
        .await
        .expect("company B should be created");

    let tenant_a = tenant_context_for_company(&company_a.id, "user-a");
    let tenant_b = tenant_context_for_company(&company_b.id, "user-b");

    let summary_a = asset_summary(&company_a.id);
    service
        .upsert_account(UpsertAccountRequest {
            account: summary_a.clone(),
            tenant: tenant_a.clone(),
        })
        .await
        .expect("summary account for company A");
    service
        .upsert_account(UpsertAccountRequest {
            account: cash_account(&company_a.id, summary_a.id.clone()),
            tenant: tenant_a.clone(),
        })
        .await
        .expect("cash account for company A");
    service
        .upsert_account(UpsertAccountRequest {
            account: revenue_account(&company_a.id),
            tenant: tenant_a.clone(),
        })
        .await
        .expect("revenue account for company A");

    let mut summary_b = asset_summary(&company_b.id);
    summary_b.id = "b-acc-assets".into();
    service
        .upsert_account(UpsertAccountRequest {
            account: summary_b.clone(),
            tenant: tenant_b.clone(),
        })
        .await
        .expect("summary account for company B");
    let mut cash_b = cash_account(&company_b.id, summary_b.id.clone());
    cash_b.id = "b-acc-cash".into();
    let cash_b_id = cash_b.id.clone();
    service
        .upsert_account(UpsertAccountRequest {
            account: cash_b.clone(),
            tenant: tenant_b.clone(),
        })
        .await
        .expect("cash account for company B");
    let mut revenue_b = revenue_account(&company_b.id);
    revenue_b.id = "b-acc-rev".into();
    let revenue_b_id = revenue_b.id.clone();
    service
        .upsert_account(UpsertAccountRequest {
            account: revenue_b.clone(),
            tenant: tenant_b.clone(),
        })
        .await
        .expect("revenue account for company B");

    let entry_a = build_post_entry(
        tenant_a.clone(),
        "je-a",
        "acc-cash",
        "acc-rev",
        5_000,
        "Company A posting",
    );
    let entry_b = build_post_entry(
        tenant_b.clone(),
        "je-b",
        &cash_b_id,
        &revenue_b_id,
        7_000,
        "Company B posting",
    );

    let posted_a = service
        .post_entry(entry_a)
        .await
        .expect("company A should post successfully");
    let posted_b = service
        .post_entry(entry_b)
        .await
        .expect("company B should post successfully");

    assert_eq!(posted_a.status, EntryStatus::Posted);
    assert_eq!(posted_b.status, EntryStatus::Posted);

    // Ensure audit events recorded for both entries.
    let events_a = service
        .list_audit_trail(AuditTrailFilter {
            entity_id: Some(posted_a.id.clone()),
            limit: None,
            cursor: None,
            tenant: tenant_a.clone(),
        })
        .await
        .expect("audit events for company A");
    assert!(
        events_a
            .iter()
            .any(|event| event.description.contains("Posted entry"))
    );

    let events_b = service
        .list_audit_trail(AuditTrailFilter {
            entity_id: Some(posted_b.id.clone()),
            limit: None,
            cursor: None,
            tenant: tenant_b.clone(),
        })
        .await
        .expect("audit events for company B");
    assert!(
        events_b
            .iter()
            .any(|event| event.description.contains("Posted entry"))
    );

    let cross_events = service
        .list_audit_trail(AuditTrailFilter {
            entity_id: Some(posted_a.id.clone()),
            limit: None,
            cursor: None,
            tenant: tenant_b,
        })
        .await
        .expect("tenant B should be able to query without error");
    assert!(
        cross_events.is_empty(),
        "tenant B must not see tenant A events"
    );
}
