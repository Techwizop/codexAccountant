use chrono::DateTime;
use chrono::Utc;
use codex_app_server_protocol::LedgerAccount;
use codex_app_server_protocol::LedgerAccountType;
use codex_app_server_protocol::LedgerAuditEvent;
use codex_app_server_protocol::LedgerCompany;
use codex_app_server_protocol::LedgerCreateCompanyParams;
use codex_app_server_protocol::LedgerCurrency;
use codex_app_server_protocol::LedgerCurrencyMode;
use codex_app_server_protocol::LedgerCurrencyRate;
use codex_app_server_protocol::LedgerEntryOrigin;
use codex_app_server_protocol::LedgerEntryStatus;
use codex_app_server_protocol::LedgerFiscalCalendar;
use codex_app_server_protocol::LedgerJournal;
use codex_app_server_protocol::LedgerJournalEntry;
use codex_app_server_protocol::LedgerJournalLine;
use codex_app_server_protocol::LedgerJournalType;
use codex_app_server_protocol::LedgerListAuditTrailParams;
use codex_app_server_protocol::LedgerListAuditTrailResponse;
use codex_app_server_protocol::LedgerLockPeriodParams;
use codex_app_server_protocol::LedgerPeriodAction;
use codex_app_server_protocol::LedgerPeriodLock;
use codex_app_server_protocol::LedgerPeriodRef;
use codex_app_server_protocol::LedgerPeriodState;
use codex_app_server_protocol::LedgerPostEntryParams;
use codex_app_server_protocol::LedgerPostingMode;
use codex_app_server_protocol::LedgerPostingSide;
use codex_app_server_protocol::LedgerReconciliationStatus;
use codex_app_server_protocol::LedgerRevalueCurrencyParams;
use codex_app_server_protocol::LedgerRevalueCurrencyResponse;
use codex_app_server_protocol::LedgerReverseEntryParams;
use codex_app_server_protocol::LedgerTaxCode;
use codex_app_server_protocol::LedgerUpsertAccountParams;
use codex_ledger::Account as LedgerAccountModel;
use codex_ledger::AccountType as LedgerAccountTypeModel;
use codex_ledger::AuditEvent as LedgerAuditEventModel;
use codex_ledger::AuditTrailFilter;
use codex_ledger::Company;
use codex_ledger::CreateCompanyRequest;
use codex_ledger::Currency as LedgerCurrencyModel;
use codex_ledger::CurrencyMode as LedgerCurrencyModeModel;
use codex_ledger::CurrencyRate as LedgerCurrencyRateModel;
use codex_ledger::CurrencyRevaluationRequest;
use codex_ledger::FiscalCalendar as LedgerFiscalCalendarModel;
use codex_ledger::Journal as LedgerJournalModel;
use codex_ledger::JournalEntry as LedgerJournalEntryModel;
use codex_ledger::JournalLine as LedgerJournalLineModel;
use codex_ledger::LedgerType as LedgerJournalTypeModel;
use codex_ledger::LockPeriodRequest;
use codex_ledger::PeriodAction as LedgerPeriodActionModel;
use codex_ledger::PeriodLockInfo;
use codex_ledger::PeriodRef as LedgerPeriodRefModel;
use codex_ledger::PeriodState as LedgerPeriodStateModel;
use codex_ledger::PostEntryRequest;
use codex_ledger::PostingMode as LedgerPostingModeModel;
use codex_ledger::PostingSide as LedgerPostingSideModel;
use codex_ledger::ReconciliationStatus as LedgerReconciliationStatusModel;
use codex_ledger::ReverseEntryRequest;
use codex_ledger::TenantContext as LedgerTenantContext;
use codex_ledger::UpsertAccountRequest;

pub fn to_ledger_currency(currency: LedgerCurrency) -> LedgerCurrencyModel {
    LedgerCurrencyModel {
        code: currency.code,
        precision: currency.precision,
    }
}

pub fn from_ledger_currency(currency: LedgerCurrencyModel) -> LedgerCurrency {
    LedgerCurrency {
        code: currency.code,
        precision: currency.precision,
    }
}

pub fn to_ledger_fiscal_calendar(calendar: LedgerFiscalCalendar) -> LedgerFiscalCalendarModel {
    LedgerFiscalCalendarModel {
        periods_per_year: calendar.periods_per_year,
        opening_month: calendar.opening_month,
    }
}

pub fn from_ledger_fiscal_calendar(calendar: LedgerFiscalCalendarModel) -> LedgerFiscalCalendar {
    LedgerFiscalCalendar {
        periods_per_year: calendar.periods_per_year,
        opening_month: calendar.opening_month,
    }
}

pub fn from_ledger_company(company: Company) -> LedgerCompany {
    LedgerCompany {
        id: company.id,
        name: company.name,
        base_currency: from_ledger_currency(company.base_currency),
        fiscal_calendar: from_ledger_fiscal_calendar(company.fiscal_calendar),
        metadata: company.metadata,
    }
}

pub fn to_ledger_account_type(account_type: LedgerAccountType) -> LedgerAccountTypeModel {
    match account_type {
        LedgerAccountType::Asset => LedgerAccountTypeModel::Asset,
        LedgerAccountType::Liability => LedgerAccountTypeModel::Liability,
        LedgerAccountType::Equity => LedgerAccountTypeModel::Equity,
        LedgerAccountType::Revenue => LedgerAccountTypeModel::Revenue,
        LedgerAccountType::Expense => LedgerAccountTypeModel::Expense,
        LedgerAccountType::OffBalance => LedgerAccountTypeModel::OffBalance,
    }
}

pub fn from_ledger_account_type(account_type: LedgerAccountTypeModel) -> LedgerAccountType {
    match account_type {
        LedgerAccountTypeModel::Asset => LedgerAccountType::Asset,
        LedgerAccountTypeModel::Liability => LedgerAccountType::Liability,
        LedgerAccountTypeModel::Equity => LedgerAccountType::Equity,
        LedgerAccountTypeModel::Revenue => LedgerAccountType::Revenue,
        LedgerAccountTypeModel::Expense => LedgerAccountType::Expense,
        LedgerAccountTypeModel::OffBalance => LedgerAccountType::OffBalance,
    }
}

pub fn to_ledger_currency_mode(mode: LedgerCurrencyMode) -> LedgerCurrencyModeModel {
    match mode {
        LedgerCurrencyMode::FunctionalOnly => LedgerCurrencyModeModel::FunctionalOnly,
        LedgerCurrencyMode::Transactional => LedgerCurrencyModeModel::Transactional,
        LedgerCurrencyMode::MultiCurrency => LedgerCurrencyModeModel::MultiCurrency,
    }
}

pub fn from_ledger_currency_mode(mode: LedgerCurrencyModeModel) -> LedgerCurrencyMode {
    match mode {
        LedgerCurrencyModeModel::FunctionalOnly => LedgerCurrencyMode::FunctionalOnly,
        LedgerCurrencyModeModel::Transactional => LedgerCurrencyMode::Transactional,
        LedgerCurrencyModeModel::MultiCurrency => LedgerCurrencyMode::MultiCurrency,
    }
}

pub fn to_ledger_tax_code(tax: LedgerTaxCode) -> codex_ledger::TaxCode {
    codex_ledger::TaxCode {
        code: tax.code,
        description: tax.description,
        rate_percent: tax.rate_percent,
    }
}

pub fn from_ledger_tax_code(tax: codex_ledger::TaxCode) -> LedgerTaxCode {
    LedgerTaxCode {
        code: tax.code,
        description: tax.description,
        rate_percent: tax.rate_percent,
    }
}

pub fn to_ledger_account(account: LedgerAccount) -> LedgerAccountModel {
    LedgerAccountModel {
        id: account.id,
        company_id: account.company_id,
        code: account.code,
        name: account.name,
        account_type: to_ledger_account_type(account.account_type),
        parent_account_id: account.parent_account_id,
        currency_mode: to_ledger_currency_mode(account.currency_mode),
        tax_code: account.tax_code.map(to_ledger_tax_code),
        is_summary: account.is_summary,
        is_active: account.is_active,
    }
}

pub fn from_ledger_account(account: LedgerAccountModel) -> LedgerAccount {
    LedgerAccount {
        id: account.id,
        company_id: account.company_id,
        code: account.code,
        name: account.name,
        account_type: from_ledger_account_type(account.account_type),
        parent_account_id: account.parent_account_id,
        currency_mode: from_ledger_currency_mode(account.currency_mode),
        tax_code: account.tax_code.map(from_ledger_tax_code),
        is_summary: account.is_summary,
        is_active: account.is_active,
    }
}

pub fn to_ledger_posting_side(side: LedgerPostingSide) -> LedgerPostingSideModel {
    match side {
        LedgerPostingSide::Debit => LedgerPostingSideModel::Debit,
        LedgerPostingSide::Credit => LedgerPostingSideModel::Credit,
    }
}

pub fn from_ledger_posting_side(side: LedgerPostingSideModel) -> LedgerPostingSide {
    match side {
        LedgerPostingSideModel::Debit => LedgerPostingSide::Debit,
        LedgerPostingSideModel::Credit => LedgerPostingSide::Credit,
    }
}

pub fn to_ledger_currency_rate(rate: LedgerCurrencyRate) -> LedgerCurrencyRateModel {
    LedgerCurrencyRateModel {
        base: to_ledger_currency(rate.base),
        quote: to_ledger_currency(rate.quote),
        rate: rate.rate,
        source: rate.source,
        observed_at: std::time::SystemTime::now(),
    }
}

pub fn from_ledger_currency_rate(rate: LedgerCurrencyRateModel) -> LedgerCurrencyRate {
    LedgerCurrencyRate {
        base: from_ledger_currency(rate.base),
        quote: from_ledger_currency(rate.quote),
        rate: rate.rate,
        source: rate.source,
    }
}

pub fn to_ledger_journal_line(line: LedgerJournalLine) -> LedgerJournalLineModel {
    LedgerJournalLineModel {
        id: line.id,
        account_id: line.account_id,
        side: to_ledger_posting_side(line.side),
        amount_minor: line.amount_minor,
        currency: to_ledger_currency(line.currency),
        functional_amount_minor: line.functional_amount_minor,
        functional_currency: to_ledger_currency(line.functional_currency),
        exchange_rate: line.exchange_rate.map(to_ledger_currency_rate),
        tax_code: line.tax_code.map(to_ledger_tax_code),
        memo: line.memo,
    }
}

pub fn from_ledger_journal_line(line: LedgerJournalLineModel) -> LedgerJournalLine {
    LedgerJournalLine {
        id: line.id,
        account_id: line.account_id,
        side: from_ledger_posting_side(line.side),
        amount_minor: line.amount_minor,
        currency: from_ledger_currency(line.currency),
        functional_amount_minor: line.functional_amount_minor,
        functional_currency: from_ledger_currency(line.functional_currency),
        exchange_rate: line.exchange_rate.map(from_ledger_currency_rate),
        tax_code: line.tax_code.map(from_ledger_tax_code),
        memo: line.memo,
    }
}

pub fn to_ledger_entry_status(status: LedgerEntryStatus) -> codex_ledger::EntryStatus {
    match status {
        LedgerEntryStatus::Draft => codex_ledger::EntryStatus::Draft,
        LedgerEntryStatus::Proposed => codex_ledger::EntryStatus::Proposed,
        LedgerEntryStatus::Posted => codex_ledger::EntryStatus::Posted,
        LedgerEntryStatus::Reversed => codex_ledger::EntryStatus::Reversed,
    }
}

pub fn from_ledger_entry_status(status: codex_ledger::EntryStatus) -> LedgerEntryStatus {
    match status {
        codex_ledger::EntryStatus::Draft => LedgerEntryStatus::Draft,
        codex_ledger::EntryStatus::Proposed => LedgerEntryStatus::Proposed,
        codex_ledger::EntryStatus::Posted => LedgerEntryStatus::Posted,
        codex_ledger::EntryStatus::Reversed => LedgerEntryStatus::Reversed,
    }
}

pub fn to_ledger_entry_origin(origin: LedgerEntryOrigin) -> codex_ledger::EntryOrigin {
    match origin {
        LedgerEntryOrigin::Manual => codex_ledger::EntryOrigin::Manual,
        LedgerEntryOrigin::Ingestion => codex_ledger::EntryOrigin::Ingestion,
        LedgerEntryOrigin::AiSuggested => codex_ledger::EntryOrigin::AiSuggested,
        LedgerEntryOrigin::Adjustment => codex_ledger::EntryOrigin::Adjustment,
    }
}

pub fn to_ledger_reconciliation_status(
    status: LedgerReconciliationStatus,
) -> LedgerReconciliationStatusModel {
    match status {
        LedgerReconciliationStatus::Unreconciled => LedgerReconciliationStatusModel::Unreconciled,
        LedgerReconciliationStatus::Pending { session_id } => {
            LedgerReconciliationStatusModel::Pending { session_id }
        }
        LedgerReconciliationStatus::Reconciled { session_id } => {
            LedgerReconciliationStatusModel::Reconciled { session_id }
        }
        LedgerReconciliationStatus::WriteOff { approval_reference } => {
            LedgerReconciliationStatusModel::WriteOff { approval_reference }
        }
    }
}

pub fn from_ledger_entry_origin(origin: codex_ledger::EntryOrigin) -> LedgerEntryOrigin {
    match origin {
        codex_ledger::EntryOrigin::Manual => LedgerEntryOrigin::Manual,
        codex_ledger::EntryOrigin::Ingestion => LedgerEntryOrigin::Ingestion,
        codex_ledger::EntryOrigin::AiSuggested => LedgerEntryOrigin::AiSuggested,
        codex_ledger::EntryOrigin::Adjustment => LedgerEntryOrigin::Adjustment,
    }
}

pub fn to_ledger_journal_entry(entry: LedgerJournalEntry) -> LedgerJournalEntryModel {
    LedgerJournalEntryModel {
        id: entry.id,
        journal_id: entry.journal_id,
        status: to_ledger_entry_status(entry.status),
        reconciliation_status: to_ledger_reconciliation_status(entry.reconciliation_status),
        lines: entry
            .lines
            .into_iter()
            .map(to_ledger_journal_line)
            .collect(),
        origin: to_ledger_entry_origin(entry.origin),
        memo: entry.memo,
        reverses_entry_id: entry.reverses_entry_id,
        reversed_by_entry_id: entry.reversed_by_entry_id,
    }
}

pub fn from_ledger_journal_entry(entry: LedgerJournalEntryModel) -> LedgerJournalEntry {
    LedgerJournalEntry {
        id: entry.id,
        journal_id: entry.journal_id,
        status: from_ledger_entry_status(entry.status),
        reconciliation_status: from_ledger_reconciliation_status(entry.reconciliation_status),
        lines: entry
            .lines
            .into_iter()
            .map(from_ledger_journal_line)
            .collect(),
        origin: from_ledger_entry_origin(entry.origin),
        memo: entry.memo,
        reverses_entry_id: entry.reverses_entry_id,
        reversed_by_entry_id: entry.reversed_by_entry_id,
    }
}

fn from_ledger_reconciliation_status(
    status: LedgerReconciliationStatusModel,
) -> LedgerReconciliationStatus {
    match status {
        LedgerReconciliationStatusModel::Unreconciled => LedgerReconciliationStatus::Unreconciled,
        LedgerReconciliationStatusModel::Pending { session_id } => {
            LedgerReconciliationStatus::Pending { session_id }
        }
        LedgerReconciliationStatusModel::Reconciled { session_id } => {
            LedgerReconciliationStatus::Reconciled { session_id }
        }
        LedgerReconciliationStatusModel::WriteOff { approval_reference } => {
            LedgerReconciliationStatus::WriteOff { approval_reference }
        }
    }
}

pub fn to_ledger_posting_mode(mode: LedgerPostingMode) -> LedgerPostingModeModel {
    match mode {
        LedgerPostingMode::DryRun => LedgerPostingModeModel::DryRun,
        LedgerPostingMode::Commit => LedgerPostingModeModel::Commit,
    }
}

pub fn to_ledger_period_action(action: LedgerPeriodAction) -> LedgerPeriodActionModel {
    match action {
        LedgerPeriodAction::SoftClose => LedgerPeriodActionModel::SoftClose,
        LedgerPeriodAction::Close => LedgerPeriodActionModel::Close,
        LedgerPeriodAction::ReopenSoft => LedgerPeriodActionModel::ReopenSoft,
        LedgerPeriodAction::ReopenFull => LedgerPeriodActionModel::ReopenFull,
    }
}

pub fn from_ledger_period_action_model(action: LedgerPeriodActionModel) -> LedgerPeriodAction {
    match action {
        LedgerPeriodActionModel::SoftClose => LedgerPeriodAction::SoftClose,
        LedgerPeriodActionModel::Close => LedgerPeriodAction::Close,
        LedgerPeriodActionModel::ReopenSoft => LedgerPeriodAction::ReopenSoft,
        LedgerPeriodActionModel::ReopenFull => LedgerPeriodAction::ReopenFull,
    }
}

pub fn to_ledger_period_ref(period: LedgerPeriodRef) -> LedgerPeriodRefModel {
    LedgerPeriodRefModel {
        fiscal_year: period.fiscal_year,
        period: period.period,
    }
}

pub fn from_ledger_journal_type(journal_type: LedgerJournalTypeModel) -> LedgerJournalType {
    match journal_type {
        LedgerJournalTypeModel::General => LedgerJournalType::General,
        LedgerJournalTypeModel::AccountsPayable => LedgerJournalType::AccountsPayable,
        LedgerJournalTypeModel::AccountsReceivable => LedgerJournalType::AccountsReceivable,
        LedgerJournalTypeModel::Cash => LedgerJournalType::Cash,
        LedgerJournalTypeModel::SubLedger => LedgerJournalType::SubLedger,
    }
}

pub fn from_ledger_period_state(state: LedgerPeriodStateModel) -> LedgerPeriodState {
    match state {
        LedgerPeriodStateModel::Open => LedgerPeriodState::Open,
        LedgerPeriodStateModel::SoftClosed => LedgerPeriodState::SoftClosed,
        LedgerPeriodStateModel::Closed => LedgerPeriodState::Closed,
    }
}

pub fn from_ledger_journal(journal: LedgerJournalModel) -> LedgerJournal {
    LedgerJournal {
        id: journal.id,
        company_id: journal.company_id,
        ledger_type: from_ledger_journal_type(journal.ledger_type),
        period_state: from_ledger_period_state(journal.period_state),
        latest_lock: journal.latest_lock.map(from_period_lock),
        lock_history: journal
            .lock_history
            .into_iter()
            .map(from_period_lock)
            .collect(),
    }
}

fn from_period_lock(lock: PeriodLockInfo) -> LedgerPeriodLock {
    let locked_at: DateTime<Utc> = lock.locked_at.into();
    LedgerPeriodLock {
        period: LedgerPeriodRef {
            fiscal_year: lock.period.fiscal_year,
            period: lock.period.period,
        },
        action: from_ledger_period_action_model(lock.action),
        approval_reference: lock.approval_reference,
        locked_at: locked_at.to_rfc3339(),
        locked_by: lock.locked_by,
    }
}

pub fn build_create_company_request(
    params: LedgerCreateCompanyParams,
    tenant: LedgerTenantContext,
) -> CreateCompanyRequest {
    CreateCompanyRequest {
        name: params.name,
        base_currency: to_ledger_currency(params.base_currency),
        fiscal_calendar: to_ledger_fiscal_calendar(params.fiscal_calendar),
        tenant,
    }
}

pub fn build_upsert_account_request(
    params: LedgerUpsertAccountParams,
    tenant: LedgerTenantContext,
) -> UpsertAccountRequest {
    UpsertAccountRequest {
        account: to_ledger_account(params.account),
        tenant,
    }
}

pub fn build_post_entry_request(
    params: LedgerPostEntryParams,
    tenant: LedgerTenantContext,
) -> PostEntryRequest {
    PostEntryRequest {
        entry: to_ledger_journal_entry(params.entry),
        mode: to_ledger_posting_mode(params.mode),
        tenant,
    }
}

pub fn build_reverse_entry_request(
    params: LedgerReverseEntryParams,
    tenant: LedgerTenantContext,
) -> ReverseEntryRequest {
    ReverseEntryRequest {
        entry_id: params.entry_id,
        reason: params.reason,
        tenant,
    }
}

pub fn build_lock_period_request(
    params: LedgerLockPeriodParams,
    tenant: LedgerTenantContext,
) -> LockPeriodRequest {
    LockPeriodRequest {
        journal_id: params.journal_id,
        period: to_ledger_period_ref(params.period),
        action: to_ledger_period_action(params.action),
        approval_reference: params.approval_reference,
        tenant,
    }
}

pub fn build_revalue_currency_request(
    params: LedgerRevalueCurrencyParams,
    tenant: LedgerTenantContext,
) -> CurrencyRevaluationRequest {
    CurrencyRevaluationRequest {
        journal_id: params.journal_id,
        period: to_ledger_period_ref(params.period),
        currencies: params
            .currencies
            .into_iter()
            .map(to_ledger_currency)
            .collect(),
        tenant,
    }
}

pub fn build_audit_trail_filter(
    params: LedgerListAuditTrailParams,
    tenant: LedgerTenantContext,
) -> AuditTrailFilter {
    AuditTrailFilter {
        entity_id: params.entity_id,
        limit: params.limit,
        cursor: params.cursor,
        tenant,
    }
}

pub fn from_ledger_audit_event(event: LedgerAuditEventModel) -> LedgerAuditEvent {
    let occurred_at: DateTime<Utc> = event.occurred_at.into();
    LedgerAuditEvent {
        id: event.id,
        company_id: event.company_id,
        entity_id: event.entity_id,
        actor: event.actor,
        occurred_at: occurred_at.to_rfc3339(),
        description: event.description,
    }
}

pub fn build_revalue_currency_response(
    entries: Vec<LedgerJournalEntryModel>,
    next_cursor: Option<String>,
) -> LedgerRevalueCurrencyResponse {
    LedgerRevalueCurrencyResponse {
        entries: entries.into_iter().map(from_ledger_journal_entry).collect(),
        next_cursor,
    }
}

pub fn build_audit_trail_response(
    events: Vec<LedgerAuditEventModel>,
    next_cursor: Option<String>,
) -> LedgerListAuditTrailResponse {
    LedgerListAuditTrailResponse {
        events: events.into_iter().map(from_ledger_audit_event).collect(),
        next_cursor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::LedgerCurrency;
    use codex_app_server_protocol::LedgerPeriodAction;
    use std::time::Duration;
    use std::time::SystemTime;

    #[test]
    fn converts_account_roundtrip() {
        let account = LedgerAccount {
            id: "acc-1".into(),
            company_id: "co-1".into(),
            code: "1000".into(),
            name: "Cash".into(),
            account_type: LedgerAccountType::Asset,
            parent_account_id: None,
            currency_mode: LedgerCurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: false,
            is_active: true,
        };

        let converted = to_ledger_account(account.clone());
        let roundtrip = from_ledger_account(converted);
        assert_eq!(account, roundtrip);
    }

    #[test]
    fn builds_lock_period_request() {
        let params = LedgerLockPeriodParams {
            company_id: "co-1".into(),
            journal_id: "jnl".into(),
            period: LedgerPeriodRef {
                fiscal_year: 2025,
                period: 3,
            },
            action: LedgerPeriodAction::SoftClose,
            approval_reference: Some("approval-1".into()),
        };
        let tenant = LedgerTenantContext {
            tenant_id: "co-1".into(),
            user_id: "svc".into(),
            roles: vec![],
            locale: None,
        };

        let request = build_lock_period_request(params, tenant);
        assert_eq!(request.period.fiscal_year, 2025);
        assert_eq!(request.action, LedgerPeriodActionModel::SoftClose);
        assert_eq!(request.approval_reference, Some("approval-1".into()));
    }

    #[test]
    fn builds_revalue_currency_request() {
        let params = LedgerRevalueCurrencyParams {
            company_id: "co-1".into(),
            journal_id: "jnl-gl".into(),
            period: LedgerPeriodRef {
                fiscal_year: 2025,
                period: 4,
            },
            currencies: vec![LedgerCurrency {
                code: "EUR".into(),
                precision: 2,
            }],
        };

        let tenant = LedgerTenantContext {
            tenant_id: "co-1".into(),
            user_id: "svc".into(),
            roles: vec![],
            locale: None,
        };

        let request = build_revalue_currency_request(params, tenant);
        assert_eq!(request.journal_id, "jnl-gl");
        assert_eq!(request.period.fiscal_year, 2025);
        assert_eq!(request.currencies.len(), 1);
        assert_eq!(request.currencies[0].code, "EUR");
    }

    #[test]
    fn formats_audit_event_timestamp() {
        let event = LedgerAuditEventModel {
            id: "audit-1".into(),
            company_id: "co-1".into(),
            entity_id: "jnl-gl".into(),
            actor: "svc".into(),
            occurred_at: SystemTime::UNIX_EPOCH + Duration::from_secs(123),
            description: "Updated period state".into(),
        };

        let converted = from_ledger_audit_event(event);
        assert_eq!(converted.occurred_at, "1970-01-01T00:02:03+00:00");
    }

    #[test]
    fn builds_audit_trail_filter_from_params() {
        let params = LedgerListAuditTrailParams {
            company_id: "co-1".into(),
            entity_id: Some("jnl-gl".into()),
            limit: Some(5),
            cursor: Some("audit-1".into()),
        };

        let tenant = LedgerTenantContext {
            tenant_id: "co-1".into(),
            user_id: "svc".into(),
            roles: vec![],
            locale: None,
        };

        let filter = build_audit_trail_filter(params, tenant);
        assert_eq!(filter.entity_id.as_deref(), Some("jnl-gl"));
        assert_eq!(filter.limit, Some(5));
        assert_eq!(filter.cursor.as_deref(), Some("audit-1"));
    }

    #[test]
    fn builds_revalue_currency_response_with_next_cursor() {
        let entry = LedgerJournalEntryModel {
            id: "je-1".into(),
            journal_id: "jnl-gl".into(),
            status: codex_ledger::EntryStatus::Draft,
            reconciliation_status: codex_ledger::ReconciliationStatus::Unreconciled,
            lines: vec![LedgerJournalLineModel {
                id: "ln-1".into(),
                account_id: "acc-1".into(),
                side: LedgerPostingSideModel::Debit,
                amount_minor: 100,
                currency: LedgerCurrencyModel {
                    code: "USD".into(),
                    precision: 2,
                },
                functional_amount_minor: 100,
                functional_currency: LedgerCurrencyModel {
                    code: "USD".into(),
                    precision: 2,
                },
                exchange_rate: None,
                tax_code: None,
                memo: None,
            }],
            origin: codex_ledger::EntryOrigin::Manual,
            memo: None,
            reverses_entry_id: None,
            reversed_by_entry_id: None,
        };

        let response = build_revalue_currency_response(vec![entry], Some("je-1".into()));
        assert_eq!(response.entries.len(), 1);
        assert_eq!(response.entries[0].id, "je-1");
        assert_eq!(response.next_cursor.as_deref(), Some("je-1"));
    }

    #[test]
    fn builds_audit_trail_response_with_events() {
        let events = vec![LedgerAuditEventModel {
            id: "audit-1".into(),
            company_id: "co-1".into(),
            entity_id: "jnl-gl".into(),
            actor: "svc".into(),
            occurred_at: SystemTime::UNIX_EPOCH,
            description: "Updated period state".into(),
        }];

        let response = build_audit_trail_response(events, None);
        assert_eq!(response.events.len(), 1);
        assert_eq!(response.events[0].description, "Updated period state");
        assert_eq!(response.next_cursor, None);
    }
}
