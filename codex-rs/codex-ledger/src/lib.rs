#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::time::SystemTime;

use async_trait::async_trait;

mod memory;

pub use memory::InMemoryLedgerService;

pub type CompanyId = String;
pub type AccountId = String;
pub type JournalId = String;
pub type JournalEntryId = String;
pub type JournalLineId = String;

pub type LedgerResult<T> = Result<T, LedgerError>;

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("operation rejected: {0}")]
    Rejected(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Company {
    pub id: CompanyId,
    pub name: String,
    pub base_currency: Currency,
    pub fiscal_calendar: FiscalCalendar,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiscalCalendar {
    pub periods_per_year: u8,
    pub opening_month: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: AccountId,
    pub company_id: CompanyId,
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub parent_account_id: Option<AccountId>,
    pub currency_mode: CurrencyMode,
    pub tax_code: Option<TaxCode>,
    pub is_summary: bool,
    pub is_active: bool,
}

impl Account {
    pub fn allows_posting(&self) -> bool {
        self.is_active && !self.is_summary
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
    OffBalance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrencyMode {
    FunctionalOnly,
    Transactional,
    MultiCurrency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Journal {
    pub id: JournalId,
    pub company_id: CompanyId,
    pub ledger_type: LedgerType,
    pub period_state: PeriodState,
    pub latest_lock: Option<PeriodLockInfo>,
    pub lock_history: Vec<PeriodLockInfo>,
}

impl Journal {
    pub fn can_post(&self, allow_soft_close_override: bool) -> bool {
        match self.period_state {
            PeriodState::Open => true,
            PeriodState::SoftClosed => allow_soft_close_override,
            PeriodState::Closed => false,
        }
    }

    #[must_use]
    pub fn lock_history(&self) -> &[PeriodLockInfo] {
        &self.lock_history
    }

    #[must_use]
    pub fn latest_lock(&self) -> Option<&PeriodLockInfo> {
        match self.lock_history.last() {
            Some(info) => Some(info),
            None => self.latest_lock.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerType {
    General,
    AccountsPayable,
    AccountsReceivable,
    Cash,
    SubLedger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodState {
    Open,
    SoftClosed,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodLockInfo {
    pub period: PeriodRef,
    pub action: PeriodAction,
    pub approval_reference: Option<String>,
    pub locked_at: SystemTime,
    pub locked_by: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    pub id: JournalEntryId,
    pub journal_id: JournalId,
    pub status: EntryStatus,
    pub reconciliation_status: ReconciliationStatus,
    pub lines: Vec<JournalLine>,
    pub origin: EntryOrigin,
    pub memo: Option<String>,
    pub reverses_entry_id: Option<JournalEntryId>,
    pub reversed_by_entry_id: Option<JournalEntryId>,
}

impl JournalEntry {
    pub fn is_balanced(&self) -> bool {
        let (debits, credits) =
            self.lines
                .iter()
                .fold((0_i64, 0_i64), |(d, c), line| match line.side {
                    PostingSide::Debit => (d + line.functional_amount_minor, c),
                    PostingSide::Credit => (d, c + line.functional_amount_minor),
                });
        debits == credits
    }

    pub fn validate(&self) -> LedgerResult<()> {
        if !self.is_balanced() {
            return Err(LedgerError::Validation("Journal entry must balance".into()));
        }
        if self
            .lines
            .iter()
            .any(|line| !line.has_currency_provenance())
        {
            return Err(LedgerError::Validation(
                "Currency amounts must include provenance".into(),
            ));
        }
        Ok(())
    }

    pub fn mark_reconciliation_pending(
        &mut self,
        session_id: impl Into<String>,
    ) -> LedgerResult<()> {
        match self.reconciliation_status {
            ReconciliationStatus::Unreconciled | ReconciliationStatus::Pending { .. } => {
                self.reconciliation_status = ReconciliationStatus::Pending {
                    session_id: session_id.into(),
                };
                Ok(())
            }
            ReconciliationStatus::Reconciled { .. } | ReconciliationStatus::WriteOff { .. } => {
                Err(LedgerError::Validation(
                    "cannot mark entry pending after reconciliation or write-off".into(),
                ))
            }
        }
    }

    pub fn mark_reconciled(&mut self, session_id: &str) -> LedgerResult<()> {
        match &self.reconciliation_status {
            ReconciliationStatus::Pending {
                session_id: pending,
            } if pending == session_id => {
                self.reconciliation_status = ReconciliationStatus::Reconciled {
                    session_id: session_id.into(),
                };
                Ok(())
            }
            ReconciliationStatus::Pending {
                session_id: pending,
            } => Err(LedgerError::Validation(format!(
                "entry is pending reconciliation under session {pending}"
            ))),
            ReconciliationStatus::Reconciled { .. } => Ok(()),
            ReconciliationStatus::Unreconciled | ReconciliationStatus::WriteOff { .. } => Err(
                LedgerError::Validation("entry must be pending before reconciliation".into()),
            ),
        }
    }

    pub fn mark_write_off(&mut self, approval_reference: impl Into<String>) -> LedgerResult<()> {
        let approval_reference = approval_reference.into();
        if approval_reference.trim().is_empty() {
            return Err(LedgerError::Validation(
                "write-off requires an approval reference".into(),
            ));
        }
        match self.reconciliation_status {
            ReconciliationStatus::Unreconciled | ReconciliationStatus::Pending { .. } => {
                self.reconciliation_status = ReconciliationStatus::WriteOff { approval_reference };
                Ok(())
            }
            ReconciliationStatus::Reconciled { .. } => Err(LedgerError::Validation(
                "reconciled entries cannot be written off".into(),
            )),
            ReconciliationStatus::WriteOff { .. } => Ok(()),
        }
    }

    pub fn clear_reconciliation(&mut self) {
        self.reconciliation_status = ReconciliationStatus::Unreconciled;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStatus {
    Draft,
    Proposed,
    Posted,
    Reversed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryOrigin {
    Manual,
    Ingestion,
    AiSuggested,
    Adjustment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationStatus {
    Unreconciled,
    Pending { session_id: String },
    Reconciled { session_id: String },
    WriteOff { approval_reference: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalLine {
    pub id: JournalLineId,
    pub account_id: AccountId,
    pub side: PostingSide,
    /// Amount expressed in the transactional currency (minor units).
    pub amount_minor: i64,
    pub currency: Currency,
    /// Amount converted into the company functional currency (minor units).
    pub functional_amount_minor: i64,
    pub functional_currency: Currency,
    pub exchange_rate: Option<CurrencyRate>,
    pub tax_code: Option<TaxCode>,
    pub memo: Option<String>,
}

impl JournalLine {
    pub fn has_currency_provenance(&self) -> bool {
        if self.currency == self.functional_currency {
            return self.exchange_rate.is_none()
                && self.amount_minor == self.functional_amount_minor;
        }

        match &self.exchange_rate {
            Some(rate) => {
                if rate.base != self.currency || rate.quote != self.functional_currency {
                    return false;
                }
                let expected = (self.amount_minor as f64) * rate.rate;
                let rounded = expected.round() as i64;
                (rounded - self.functional_amount_minor).abs() <= 2 && rate.source.is_some()
            }
            None => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostingSide {
    Debit,
    Credit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Currency {
    pub code: String,
    pub precision: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrencyRate {
    pub base: Currency,
    pub quote: Currency,
    pub rate: f64,
    pub source: Option<String>,
    pub observed_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaxCode {
    pub code: String,
    pub description: String,
    pub rate_percent: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    pub tenant_id: CompanyId,
    pub user_id: String,
    pub roles: Vec<Role>,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Accountant,
    Reviewer,
    Auditor,
    ServiceAccount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: String,
    pub company_id: CompanyId,
    pub entity_id: String,
    pub actor: String,
    pub occurred_at: SystemTime,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodRef {
    pub fiscal_year: i32,
    pub period: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodAction {
    SoftClose,
    Close,
    ReopenSoft,
    ReopenFull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCompanyRequest {
    pub name: String,
    pub base_currency: Currency,
    pub fiscal_calendar: FiscalCalendar,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpsertAccountRequest {
    pub account: Account,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChartAccount {
    pub code: String,
    pub name: String,
    pub account_type: AccountType,
    pub parent_code: Option<String>,
    pub currency_mode: CurrencyMode,
    pub tax_code: Option<TaxCode>,
    pub is_summary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeedChartRequest {
    pub company_id: CompanyId,
    pub accounts: Vec<ChartAccount>,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostEntryRequest {
    pub entry: JournalEntry,
    pub tenant: TenantContext,
    pub mode: PostingMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostingMode {
    DryRun,
    Commit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseEntryRequest {
    pub entry_id: JournalEntryId,
    pub reason: String,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockPeriodRequest {
    pub journal_id: JournalId,
    pub period: PeriodRef,
    pub action: PeriodAction,
    pub approval_reference: Option<String>,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnsurePeriodRequest {
    pub journal_id: JournalId,
    pub period: PeriodRef,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyRevaluationRequest {
    pub journal_id: JournalId,
    pub period: PeriodRef,
    pub currencies: Vec<Currency>,
    pub tenant: TenantContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditTrailFilter {
    pub entity_id: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
    pub tenant: TenantContext,
}

#[async_trait]
pub trait LedgerService: Send + Sync {
    async fn create_company(&self, request: CreateCompanyRequest) -> LedgerResult<Company>;
    async fn upsert_account(&self, request: UpsertAccountRequest) -> LedgerResult<Account>;
    async fn seed_chart(&self, request: SeedChartRequest) -> LedgerResult<Vec<Account>>;
    async fn post_entry(&self, request: PostEntryRequest) -> LedgerResult<JournalEntry>;
    async fn reverse_entry(&self, request: ReverseEntryRequest) -> LedgerResult<JournalEntry>;
    async fn lock_period(&self, request: LockPeriodRequest) -> LedgerResult<Journal>;
    async fn ensure_period(&self, request: EnsurePeriodRequest) -> LedgerResult<Journal>;
    async fn revalue_currency(
        &self,
        request: CurrencyRevaluationRequest,
    ) -> LedgerResult<Vec<JournalEntry>>;
    async fn list_audit_trail(&self, filter: AuditTrailFilter) -> LedgerResult<Vec<AuditEvent>>;
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn journal_entry_double_entry_balance_passes() {
        let entry = JournalEntry {
            id: "je-1".into(),
            journal_id: "jnl-1".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            origin: EntryOrigin::Manual,
            memo: None,
            reverses_entry_id: None,
            reversed_by_entry_id: None,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "cash".into(),
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
                    account_id: "revenue".into(),
                    side: PostingSide::Credit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: None,
                },
            ],
        };

        assert!(entry.is_balanced());
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn journal_entry_double_entry_balance_fails() {
        let entry = JournalEntry {
            id: "je-2".into(),
            journal_id: "jnl-1".into(),
            status: EntryStatus::Draft,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            origin: EntryOrigin::Manual,
            memo: None,
            reverses_entry_id: None,
            reversed_by_entry_id: None,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "cash".into(),
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
                    account_id: "revenue".into(),
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
        };

        assert!(!entry.is_balanced());
        assert!(matches!(entry.validate(), Err(LedgerError::Validation(_))));
    }

    #[test]
    fn soft_closed_period_requires_override() {
        let journal = Journal {
            id: "jnl-1".into(),
            company_id: "comp-1".into(),
            ledger_type: LedgerType::General,
            period_state: PeriodState::SoftClosed,
            latest_lock: None,
            lock_history: Vec::new(),
        };

        assert!(!journal.can_post(false));
        assert!(journal.can_post(true));

        let closed = Journal {
            period_state: PeriodState::Closed,
            ..journal
        };
        assert!(!closed.can_post(true));
    }

    #[test]
    fn account_restrictions_block_summary_accounts() {
        let summary_account = Account {
            id: "acc-1".into(),
            company_id: "comp-1".into(),
            code: "1000".into(),
            name: "Assets".into(),
            account_type: AccountType::Asset,
            parent_account_id: None,
            currency_mode: CurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: true,
            is_active: true,
        };

        let postable = Account {
            is_summary: false,
            ..summary_account.clone()
        };

        assert!(!summary_account.allows_posting());
        assert!(postable.allows_posting());
    }

    #[test]
    fn currency_provenance_requires_rate_metadata() {
        let mut line = JournalLine {
            id: "ln-1".into(),
            account_id: "cash".into(),
            side: PostingSide::Debit,
            amount_minor: 10_000,
            currency: eur(),
            functional_amount_minor: 10_700,
            functional_currency: usd(),
            exchange_rate: None,
            tax_code: None,
            memo: None,
        };

        assert!(!line.has_currency_provenance());

        line.exchange_rate = Some(CurrencyRate {
            base: eur(),
            quote: usd(),
            rate: 1.07,
            source: Some("ECB".into()),
            observed_at: SystemTime::now(),
        });

        assert!(line.has_currency_provenance());
    }

    #[test]
    fn reconciliation_status_transitions_enforced() {
        let mut entry = JournalEntry {
            id: "je-3".into(),
            journal_id: "jnl-1".into(),
            status: EntryStatus::Posted,
            reconciliation_status: ReconciliationStatus::Unreconciled,
            origin: EntryOrigin::Manual,
            memo: None,
            reverses_entry_id: None,
            reversed_by_entry_id: None,
            lines: vec![
                JournalLine {
                    id: "ln-1".into(),
                    account_id: "cash".into(),
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
                    account_id: "revenue".into(),
                    side: PostingSide::Credit,
                    amount_minor: 10_000,
                    currency: usd(),
                    functional_amount_minor: 10_000,
                    functional_currency: usd(),
                    exchange_rate: None,
                    tax_code: None,
                    memo: None,
                },
            ],
        };

        entry
            .mark_reconciliation_pending("sess-1")
            .expect("can mark pending");
        assert!(matches!(
            entry.reconciliation_status,
            ReconciliationStatus::Pending { ref session_id } if session_id == "sess-1"
        ));

        entry
            .mark_reconciled("sess-1")
            .expect("can mark reconciled");
        assert!(matches!(
            entry.reconciliation_status,
            ReconciliationStatus::Reconciled { ref session_id } if session_id == "sess-1"
        ));
        assert!(entry.mark_write_off("APR-1").is_err());

        entry.clear_reconciliation();
        entry
            .mark_write_off("APR-1")
            .expect("can write off with approval");
        assert!(matches!(
            entry.reconciliation_status,
            ReconciliationStatus::WriteOff { ref approval_reference }
                if approval_reference == "APR-1"
        ));
        assert!(entry.mark_reconciliation_pending("sess-2").is_err());
        assert!(entry.mark_reconciled("sess-2").is_err());
    }
}
