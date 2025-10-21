use std::collections::HashMap;
use std::time::SystemTime;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::Account;
use crate::AccountId;
use crate::AuditEvent;
use crate::AuditTrailFilter;
use crate::Company;
use crate::CompanyId;
use crate::CreateCompanyRequest;
use crate::CurrencyRevaluationRequest;
use crate::EnsurePeriodRequest;
use crate::EntryOrigin;
use crate::EntryStatus;
use crate::Journal;
use crate::JournalEntry;
use crate::JournalEntryId;
use crate::JournalId;
use crate::JournalLine;
use crate::LedgerError;
use crate::LedgerResult;
use crate::LedgerService;
use crate::LedgerType;
use crate::LockPeriodRequest;
use crate::PeriodAction;
use crate::PeriodLockInfo;
use crate::PeriodRef;
use crate::PeriodState;
use crate::PostEntryRequest;
use crate::PostingMode;
use crate::PostingSide;
use crate::ReconciliationStatus;
use crate::ReverseEntryRequest;
use crate::SeedChartRequest;
use crate::UpsertAccountRequest;

/// In-memory `LedgerService` used by tests to validate the async contract.
///
/// The implementation is intentionally minimal and only supports the flows
/// exercised by the contract tests. It uses a `tokio::sync::Mutex` to guard
/// access to its state which is acceptable for the lightweight test scenarios.
#[derive(Default)]
pub struct InMemoryLedgerService {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    company_seq: u64,
    companies: HashMap<CompanyId, Company>,
    accounts: HashMap<AccountId, Account>,
    account_codes: HashMap<CompanyId, HashMap<String, AccountId>>,
    journals: HashMap<(CompanyId, JournalId), Journal>,
    periods: HashMap<(CompanyId, JournalId, i32, u8), PeriodState>,
    entries: HashMap<JournalEntryId, JournalEntry>,
    entry_companies: HashMap<JournalEntryId, CompanyId>,
    audit_events: Vec<AuditEvent>,
    audit_seq: u64,
}

fn validate_entry(entry: &JournalEntry) -> LedgerResult<()> {
    if !entry.is_balanced() {
        return Err(LedgerError::Validation("Journal entry must balance".into()));
    }

    for line in &entry.lines {
        if line.currency == line.functional_currency {
            if line.exchange_rate.is_some() || line.amount_minor != line.functional_amount_minor {
                return Err(LedgerError::Validation(
                    "Currency amounts must include provenance".into(),
                ));
            }
            continue;
        }

        let rate = match &line.exchange_rate {
            Some(rate) => rate,
            None => {
                return Err(LedgerError::Validation(
                    "Currency amounts must include provenance".into(),
                ));
            }
        };

        if rate.base != line.currency
            || rate.quote != line.functional_currency
            || rate.source.is_none()
        {
            return Err(LedgerError::Validation(
                "Currency amounts must include provenance".into(),
            ));
        }

        let expected = (line.amount_minor as f64) * rate.rate;
        let rounded = expected.round() as i64;
        if (rounded - line.functional_amount_minor).abs() > 2 {
            return Err(LedgerError::Validation(
                "Currency amounts must include provenance".into(),
            ));
        }
    }

    Ok(())
}

impl InMemoryLedgerService {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_company_id(state: &mut State) -> CompanyId {
        state.company_seq += 1;
        format!("co-{}", state.company_seq)
    }

    fn ensure_company_exists(state: &State, company_id: &CompanyId) -> LedgerResult<()> {
        if state.companies.contains_key(company_id) {
            Ok(())
        } else {
            Err(LedgerError::NotFound(format!("company {company_id}")))
        }
    }

    fn journal_key(company_id: &CompanyId, journal_id: &JournalId) -> (CompanyId, JournalId) {
        (company_id.clone(), journal_id.clone())
    }

    fn period_key(
        company_id: &CompanyId,
        journal_id: &JournalId,
        period: PeriodRef,
    ) -> (CompanyId, JournalId, i32, u8) {
        (
            company_id.clone(),
            journal_id.clone(),
            period.fiscal_year,
            period.period,
        )
    }

    fn make_account_id(company_id: &CompanyId, code: &str) -> AccountId {
        format!("acc-{company_id}-{code}")
    }

    fn record_audit_event(
        state: &mut State,
        company_id: CompanyId,
        entity_id: String,
        actor: Option<String>,
        description: String,
    ) {
        state.audit_seq += 1;
        state.audit_events.push(AuditEvent {
            id: format!("audit-{}", state.audit_seq),
            company_id,
            entity_id,
            actor: actor.unwrap_or_else(|| "system".into()),
            occurred_at: SystemTime::now(),
            description,
        });
    }
}

#[async_trait]
impl LedgerService for InMemoryLedgerService {
    async fn create_company(&self, request: CreateCompanyRequest) -> LedgerResult<Company> {
        let mut state = self.state.lock().await;
        let id = Self::next_company_id(&mut state);
        let company = Company {
            id: id.clone(),
            name: request.name,
            base_currency: request.base_currency,
            fiscal_calendar: request.fiscal_calendar,
            metadata: None,
        };

        state.companies.insert(id.clone(), company.clone());
        state.account_codes.entry(id.clone()).or_default();

        // Seed an open general ledger journal for the contract tests.
        let journal_id = "jnl-gl".to_string();
        let journal = Journal {
            id: journal_id.clone(),
            company_id: id.clone(),
            ledger_type: LedgerType::General,
            period_state: PeriodState::Open,
            latest_lock: None,
            lock_history: Vec::new(),
        };
        let key = Self::journal_key(&id, &journal_id);
        state.journals.insert(key, journal);

        Ok(company)
    }

    async fn upsert_account(&self, request: UpsertAccountRequest) -> LedgerResult<Account> {
        let mut state = self.state.lock().await;
        let account = request.account;

        Self::ensure_company_exists(&state, &account.company_id)?;

        if let Some(existing_id) = state
            .account_codes
            .get(&account.company_id)
            .and_then(|codes| codes.get(&account.code))
        {
            if existing_id != &account.id {
                return Err(LedgerError::Validation("duplicate account code".into()));
            }

            return Err(LedgerError::Validation(
                "account code already exists".into(),
            ));
        }

        if state.accounts.contains_key(&account.id) {
            return Err(LedgerError::Validation(
                "account identifier already exists".into(),
            ));
        }

        if !account.is_summary && !account.allows_posting() {
            return Err(LedgerError::Validation(
                "account must be active and non-summary".into(),
            ));
        }

        if let Some(parent_id) = &account.parent_account_id {
            match state.accounts.get(parent_id) {
                Some(parent) if parent.is_summary => {}
                Some(_) => {
                    return Err(LedgerError::Validation(
                        "parent account must be summary".into(),
                    ));
                }
                None => {
                    return Err(LedgerError::NotFound(format!("parent account {parent_id}")));
                }
            }
        }

        state.accounts.insert(account.id.clone(), account.clone());
        state
            .account_codes
            .entry(account.company_id.clone())
            .or_insert_with(HashMap::new)
            .insert(account.code.clone(), account.id.clone());

        Ok(account)
    }

    async fn post_entry(&self, request: PostEntryRequest) -> LedgerResult<JournalEntry> {
        let mut state = self.state.lock().await;
        let mut entry = request.entry;

        if entry.lines.is_empty() {
            return Err(LedgerError::Validation(
                "journal entry must contain at least one line".into(),
            ));
        }

        let mut company_id: Option<CompanyId> = None;
        for line in &entry.lines {
            let account = state
                .accounts
                .get(&line.account_id)
                .ok_or_else(|| LedgerError::NotFound(format!("account {}", line.account_id)))?;

            let account_company = account.company_id.clone();
            match &mut company_id {
                Some(existing) => {
                    if existing != &account_company {
                        return Err(LedgerError::Validation(
                            "all journal entry lines must belong to the same company".into(),
                        ));
                    }
                }
                None => {
                    company_id = Some(account_company);
                }
            }

            if !account.allows_posting() {
                return Err(LedgerError::Validation(
                    "cannot post to summary or inactive account".into(),
                ));
            }
        }

        let company_id = company_id.ok_or_else(|| {
            LedgerError::Validation("journal entry must contain at least one line".into())
        })?;
        let journal = state
            .journals
            .get_mut(&Self::journal_key(&company_id, &entry.journal_id))
            .ok_or_else(|| LedgerError::NotFound(format!("journal {}", entry.journal_id)))?;

        match journal.period_state {
            PeriodState::Open => {}
            PeriodState::SoftClosed => {
                return Err(LedgerError::Rejected(
                    "soft-close prevents posting without override".into(),
                ));
            }
            PeriodState::Closed => {
                return Err(LedgerError::Rejected("period closed".into()));
            }
        }

        validate_entry(&entry)?;
        entry.reverses_entry_id = None;
        entry.reversed_by_entry_id = None;
        entry.reconciliation_status = ReconciliationStatus::Unreconciled;

        match request.mode {
            PostingMode::DryRun => {
                let mut preview = entry;
                preview.status = EntryStatus::Proposed;
                preview.reverses_entry_id = None;
                preview.reversed_by_entry_id = None;
                Ok(preview)
            }
            PostingMode::Commit => {
                entry.status = EntryStatus::Posted;
                state.entries.insert(entry.id.clone(), entry.clone());
                state
                    .entry_companies
                    .insert(entry.id.clone(), company_id.clone());
                Self::record_audit_event(
                    &mut state,
                    company_id,
                    entry.id.clone(),
                    Some(request.tenant.user_id.clone()),
                    format!("Posted entry {}", entry.journal_id),
                );
                Ok(entry)
            }
        }
    }

    async fn reverse_entry(&self, request: ReverseEntryRequest) -> LedgerResult<JournalEntry> {
        let ReverseEntryRequest {
            entry_id,
            reason,
            tenant,
        } = request;

        let mut state = self.state.lock().await;
        let sequence = state.entries.len() + 1;

        if !state.entries.contains_key(&entry_id) {
            return Err(LedgerError::NotFound(format!("entry {entry_id}")));
        }

        let company_id = state
            .entry_companies
            .get(&entry_id)
            .cloned()
            .ok_or_else(|| {
                LedgerError::Internal(format!("missing company mapping for entry {entry_id}"))
            })?;

        let (original_id, new_entry_id, reversal_memo, reversing_entry) = {
            let entry = state.entries.get_mut(&entry_id).ok_or_else(|| {
                LedgerError::Internal(format!("entry {entry_id} missing during reversal"))
            })?;

            if entry.status != EntryStatus::Posted {
                return Err(LedgerError::Rejected("entry is not posted".into()));
            }

            if entry.reversed_by_entry_id.is_some() {
                return Err(LedgerError::Rejected("entry already reversed".into()));
            }

            let original_id = entry.id.clone();
            let new_entry_id = format!("{original_id}-rev-{sequence}");
            let reversing_lines = entry
                .lines
                .iter()
                .map(|line| JournalLine {
                    id: format!("{}-rev", line.id),
                    account_id: line.account_id.clone(),
                    side: match line.side {
                        PostingSide::Debit => PostingSide::Credit,
                        PostingSide::Credit => PostingSide::Debit,
                    },
                    amount_minor: line.amount_minor,
                    currency: line.currency.clone(),
                    functional_amount_minor: line.functional_amount_minor,
                    functional_currency: line.functional_currency.clone(),
                    exchange_rate: line.exchange_rate.clone(),
                    tax_code: line.tax_code.clone(),
                    memo: line.memo.clone(),
                })
                .collect();

            let reversal_memo = format!("Reversal of {original_id}: {reason}");
            let reversing_entry = JournalEntry {
                id: new_entry_id.clone(),
                journal_id: entry.journal_id.clone(),
                status: EntryStatus::Posted,
                reconciliation_status: ReconciliationStatus::Unreconciled,
                lines: reversing_lines,
                origin: EntryOrigin::Adjustment,
                memo: Some(reversal_memo.clone()),
                reverses_entry_id: Some(original_id.clone()),
                reversed_by_entry_id: None,
            };

            entry.reversed_by_entry_id = Some(new_entry_id.clone());

            (original_id, new_entry_id, reversal_memo, reversing_entry)
        };

        state
            .entries
            .insert(new_entry_id.clone(), reversing_entry.clone());
        state
            .entry_companies
            .insert(new_entry_id.clone(), company_id.clone());

        Self::record_audit_event(
            &mut state,
            company_id.clone(),
            original_id,
            Some(tenant.user_id.clone()),
            format!("Reversal requested: {reason}"),
        );
        Self::record_audit_event(
            &mut state,
            company_id,
            new_entry_id,
            Some(tenant.user_id),
            reversal_memo,
        );

        Ok(reversing_entry)
    }

    async fn seed_chart(&self, request: SeedChartRequest) -> LedgerResult<Vec<Account>> {
        let mut state = self.state.lock().await;
        Self::ensure_company_exists(&state, &request.company_id)?;

        let mut new_codes: HashMap<String, AccountId> = HashMap::new();
        let mut staged = Vec::new();

        for template in request.accounts {
            if state
                .account_codes
                .get(&request.company_id)
                .and_then(|codes| codes.get(&template.code))
                .is_some()
                || new_codes.contains_key(&template.code)
            {
                return Err(LedgerError::Validation(format!(
                    "account code {} already exists",
                    template.code
                )));
            }

            let parent_id = if let Some(parent_code) = &template.parent_code {
                let existing = state
                    .account_codes
                    .get(&request.company_id)
                    .and_then(|codes| codes.get(parent_code))
                    .or_else(|| new_codes.get(parent_code))
                    .ok_or_else(|| {
                        LedgerError::NotFound(format!("parent account {parent_code}"))
                    })?;
                Some(existing.clone())
            } else {
                None
            };

            let account_id = Self::make_account_id(&request.company_id, &template.code);
            if state.accounts.contains_key(&account_id) {
                return Err(LedgerError::Validation(format!(
                    "account {account_id} already exists"
                )));
            }

            let account = Account {
                id: account_id.clone(),
                company_id: request.company_id.clone(),
                code: template.code.clone(),
                name: template.name.clone(),
                account_type: template.account_type,
                parent_account_id: parent_id,
                currency_mode: template.currency_mode,
                tax_code: template.tax_code.clone(),
                is_summary: template.is_summary,
                is_active: true,
            };

            if !account.is_summary && !account.allows_posting() {
                return Err(LedgerError::Validation(
                    "account must be active and non-summary".into(),
                ));
            }

            new_codes.insert(template.code.clone(), account.id.clone());
            staged.push(account);
        }

        {
            let codes = state
                .account_codes
                .entry(request.company_id.clone())
                .or_default();
            for account in &staged {
                codes.insert(account.code.clone(), account.id.clone());
            }
        }

        for account in &staged {
            state.accounts.insert(account.id.clone(), account.clone());
        }

        Ok(staged)
    }

    async fn lock_period(&self, request: LockPeriodRequest) -> LedgerResult<Journal> {
        let mut state = self.state.lock().await;
        let journal_key = Self::journal_key(&request.tenant.tenant_id, &request.journal_id);
        let journal_entry = state
            .journals
            .get_mut(&journal_key)
            .ok_or_else(|| LedgerError::NotFound(format!("journal {}", request.journal_id)))?;

        let period_state = match request.action {
            PeriodAction::SoftClose => PeriodState::SoftClosed,
            PeriodAction::Close => PeriodState::Closed,
            PeriodAction::ReopenSoft => PeriodState::SoftClosed,
            PeriodAction::ReopenFull => PeriodState::Open,
        };

        journal_entry.period_state = period_state;
        let lock_record = PeriodLockInfo {
            period: request.period.clone(),
            action: request.action,
            approval_reference: request.approval_reference.clone(),
            locked_at: SystemTime::now(),
            locked_by: request.tenant.user_id.clone(),
        };
        journal_entry.lock_history.push(lock_record.clone());
        journal_entry.latest_lock = Some(lock_record);
        let updated = journal_entry.clone();

        let period_ref = request.period.clone();
        state.periods.insert(
            Self::period_key(
                &request.tenant.tenant_id,
                &request.journal_id,
                period_ref.clone(),
            ),
            period_state,
        );

        Self::record_audit_event(
            &mut state,
            request.tenant.tenant_id.clone(),
            request.journal_id.clone(),
            Some(request.tenant.user_id.clone()),
            format!(
                "Journal period {period_ref:?} set to {period_state:?} (approval {:?})",
                request.approval_reference
            ),
        );

        Ok(updated)
    }

    async fn ensure_period(&self, request: EnsurePeriodRequest) -> LedgerResult<Journal> {
        let mut state = self.state.lock().await;
        let company_id = request.tenant.tenant_id.clone();
        Self::ensure_company_exists(&state, &company_id)?;

        let key = Self::journal_key(&company_id, &request.journal_id);
        let base = state
            .journals
            .get(&key)
            .cloned()
            .ok_or_else(|| LedgerError::NotFound(format!("journal {}", request.journal_id)))?;

        let period_key = Self::period_key(&company_id, &request.journal_id, request.period);
        let state_entry = state.periods.entry(period_key).or_insert(PeriodState::Open);

        Ok(Journal {
            period_state: *state_entry,
            ..base
        })
    }

    async fn revalue_currency(
        &self,
        _request: CurrencyRevaluationRequest,
    ) -> LedgerResult<Vec<JournalEntry>> {
        Ok(Vec::new())
    }

    async fn list_audit_trail(&self, filter: AuditTrailFilter) -> LedgerResult<Vec<AuditEvent>> {
        let state = self.state.lock().await;
        let mut events: Vec<AuditEvent> = state.audit_events.clone();

        events.retain(|event| event.company_id == filter.tenant.tenant_id);

        if let Some(entity) = &filter.entity_id {
            events.retain(|event| &event.entity_id == entity);
        }

        if let Some(cursor) = &filter.cursor
            && let Some(pos) = events.iter().position(|event| &event.id == cursor)
        {
            events.drain(0..=pos);
        }

        if let Some(limit) = filter.limit
            && events.len() > limit
        {
            events.truncate(limit);
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AccountType;
    use crate::ChartAccount;
    use crate::Currency;
    use crate::CurrencyMode;
    use crate::FiscalCalendar;
    use crate::PeriodRef;
    use crate::Role;
    use crate::TenantContext;

    fn usd() -> Currency {
        Currency {
            code: "USD".into(),
            precision: 2,
        }
    }

    fn tenant(company_id: &str) -> TenantContext {
        TenantContext {
            tenant_id: company_id.into(),
            user_id: "tester".into(),
            roles: vec![Role::Admin],
            locale: None,
        }
    }

    async fn seed_company(service: &InMemoryLedgerService) -> Company {
        service
            .create_company(CreateCompanyRequest {
                name: "Demo".into(),
                base_currency: usd(),
                fiscal_calendar: FiscalCalendar {
                    periods_per_year: 12,
                    opening_month: 1,
                },
                tenant: tenant("seed"),
            })
            .await
            .expect("create company")
    }

    #[tokio::test]
    async fn seed_chart_creates_accounts() {
        let service = InMemoryLedgerService::new();
        let company = seed_company(&service).await;

        let accounts = service
            .seed_chart(SeedChartRequest {
                company_id: company.id.clone(),
                tenant: tenant(&company.id),
                accounts: vec![
                    ChartAccount {
                        code: "1000".into(),
                        name: "Assets".into(),
                        account_type: AccountType::Asset,
                        parent_code: None,
                        currency_mode: CurrencyMode::FunctionalOnly,
                        tax_code: None,
                        is_summary: true,
                    },
                    ChartAccount {
                        code: "1100".into(),
                        name: "Cash".into(),
                        account_type: AccountType::Asset,
                        parent_code: Some("1000".into()),
                        currency_mode: CurrencyMode::FunctionalOnly,
                        tax_code: None,
                        is_summary: false,
                    },
                ],
            })
            .await
            .expect("seed chart");

        assert_eq!(accounts.len(), 2);
        assert!(accounts.first().unwrap().is_summary);
        assert_eq!(
            accounts[1].parent_account_id.as_deref(),
            Some(accounts[0].id.as_str())
        );
    }

    #[tokio::test]
    async fn ensure_period_tracks_state() {
        let service = InMemoryLedgerService::new();
        let company = seed_company(&service).await;

        let period = PeriodRef {
            fiscal_year: 2025,
            period: 3,
        };

        let journal = service
            .ensure_period(EnsurePeriodRequest {
                journal_id: "jnl-gl".into(),
                period: period.clone(),
                tenant: tenant(&company.id),
            })
            .await
            .expect("ensure period");
        assert_eq!(journal.period_state, PeriodState::Open);

        let locked = service
            .lock_period(LockPeriodRequest {
                journal_id: "jnl-gl".into(),
                period: period.clone(),
                action: PeriodAction::SoftClose,
                approval_reference: None,
                tenant: tenant(&company.id),
            })
            .await
            .expect("lock period");
        assert_eq!(locked.period_state, PeriodState::SoftClosed);
        assert_eq!(locked.lock_history.len(), 1);
        let lock_info = locked
            .latest_lock
            .expect("lock metadata should be recorded");
        assert_eq!(lock_info.period, period);
        assert_eq!(lock_info.action, PeriodAction::SoftClose);
        assert!(lock_info.approval_reference.is_none());

        let reopened = service
            .lock_period(LockPeriodRequest {
                journal_id: "jnl-gl".into(),
                period: period.clone(),
                action: PeriodAction::ReopenFull,
                approval_reference: Some("APR-1".into()),
                tenant: tenant(&company.id),
            })
            .await
            .expect("reopen period");
        assert_eq!(reopened.period_state, PeriodState::Open);
        assert_eq!(reopened.lock_history.len(), 2);
        let latest = reopened
            .latest_lock
            .expect("latest lock should be captured");
        assert_eq!(latest.action, PeriodAction::ReopenFull);
        assert_eq!(latest.approval_reference.as_deref(), Some("APR-1"));

        let ensured = service
            .ensure_period(EnsurePeriodRequest {
                journal_id: "jnl-gl".into(),
                period: period.clone(),
                tenant: tenant(&company.id),
            })
            .await
            .expect("ensure period after reopen");
        assert_eq!(ensured.period_state, PeriodState::Open);
        assert_eq!(ensured.lock_history.len(), 2);
    }
}
