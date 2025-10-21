use std::sync::Arc;

use codex_app_server_protocol::LedgerCreateCompanyParams;
use codex_app_server_protocol::LedgerCreateCompanyResponse;
use codex_app_server_protocol::LedgerListAuditTrailParams;
use codex_app_server_protocol::LedgerListAuditTrailResponse;
use codex_app_server_protocol::LedgerLockPeriodParams;
use codex_app_server_protocol::LedgerLockPeriodResponse;
use codex_app_server_protocol::LedgerPostEntryParams;
use codex_app_server_protocol::LedgerPostEntryResponse;
use codex_app_server_protocol::LedgerRevalueCurrencyParams;
use codex_app_server_protocol::LedgerRevalueCurrencyResponse;
use codex_app_server_protocol::LedgerReverseEntryParams;
use codex_app_server_protocol::LedgerReverseEntryResponse;
use codex_app_server_protocol::LedgerUpsertAccountParams;
use codex_app_server_protocol::LedgerUpsertAccountResponse;
use codex_ledger::Account;
use codex_ledger::EnsurePeriodRequest;
use codex_ledger::Journal;
use codex_ledger::LedgerError;
use codex_ledger::LedgerResult;
use codex_ledger::LedgerService;
use codex_ledger::SeedChartRequest;
use codex_ledger::TenantContext as LedgerTenantContext;

use crate::AccountingTelemetry;
use crate::convert::build_audit_trail_filter;
use crate::convert::build_audit_trail_response;
use crate::convert::build_create_company_request;
use crate::convert::build_lock_period_request;
use crate::convert::build_post_entry_request;
use crate::convert::build_revalue_currency_request;
use crate::convert::build_revalue_currency_response;
use crate::convert::build_reverse_entry_request;
use crate::convert::build_upsert_account_request;
use crate::convert::from_ledger_account;
use crate::convert::from_ledger_company;
use crate::convert::from_ledger_journal;
use crate::convert::from_ledger_journal_entry;

#[derive(Clone)]
pub struct LedgerFacade {
    service: Arc<dyn LedgerService>,
    telemetry: Option<Arc<AccountingTelemetry>>,
}

impl LedgerFacade {
    pub fn new(service: Arc<dyn LedgerService>) -> Self {
        Self::with_telemetry(service, None)
    }

    pub fn with_telemetry(
        service: Arc<dyn LedgerService>,
        telemetry: Option<Arc<AccountingTelemetry>>,
    ) -> Self {
        Self { service, telemetry }
    }

    pub async fn create_company(
        &self,
        params: LedgerCreateCompanyParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerCreateCompanyResponse> {
        let request = build_create_company_request(params, tenant);
        self.service
            .create_company(request)
            .await
            .map(|company| LedgerCreateCompanyResponse {
                company: from_ledger_company(company),
            })
    }

    pub async fn upsert_account(
        &self,
        params: LedgerUpsertAccountParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerUpsertAccountResponse> {
        let request = build_upsert_account_request(params, tenant);
        self.service
            .upsert_account(request)
            .await
            .map(|account| LedgerUpsertAccountResponse {
                account: from_ledger_account(account),
            })
    }

    pub async fn post_entry(
        &self,
        params: LedgerPostEntryParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerPostEntryResponse> {
        let request = build_post_entry_request(params, tenant);
        self.service
            .post_entry(request)
            .await
            .map(|entry| LedgerPostEntryResponse {
                entry: from_ledger_journal_entry(entry),
            })
    }

    pub async fn seed_chart(&self, request: SeedChartRequest) -> LedgerResult<Vec<Account>> {
        self.service.seed_chart(request).await
    }

    pub async fn ensure_period(&self, request: EnsurePeriodRequest) -> LedgerResult<Journal> {
        self.service.ensure_period(request).await
    }

    pub async fn reverse_entry(
        &self,
        params: LedgerReverseEntryParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerReverseEntryResponse> {
        let request = build_reverse_entry_request(params, tenant);
        self.service
            .reverse_entry(request)
            .await
            .map(|entry| LedgerReverseEntryResponse {
                entry: from_ledger_journal_entry(entry),
            })
    }

    pub async fn lock_period(
        &self,
        params: LedgerLockPeriodParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerLockPeriodResponse> {
        if tenant.tenant_id != params.company_id {
            return Err(LedgerError::Validation(
                "tenant/company mismatch for lock period".to_string(),
            ));
        }

        let action = params.action;
        let request = build_lock_period_request(params, tenant);
        let response =
            self.service
                .lock_period(request)
                .await
                .map(|journal| LedgerLockPeriodResponse {
                    journal: from_ledger_journal(journal),
                })?;
        if let Some(telemetry) = &self.telemetry {
            telemetry.record_period_lock(action);
        }
        Ok(response)
    }

    pub async fn revalue_currency(
        &self,
        params: LedgerRevalueCurrencyParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerRevalueCurrencyResponse> {
        if tenant.tenant_id != params.company_id {
            return Err(LedgerError::Validation(
                "tenant/company mismatch for revaluation".to_string(),
            ));
        }

        let request = build_revalue_currency_request(params, tenant);
        self.service.revalue_currency(request).await.map(|entries| {
            let next_cursor = entries.last().map(|entry| entry.id.clone());
            build_revalue_currency_response(entries, next_cursor)
        })
    }

    pub async fn list_audit_trail(
        &self,
        params: LedgerListAuditTrailParams,
        tenant: LedgerTenantContext,
    ) -> LedgerResult<LedgerListAuditTrailResponse> {
        if tenant.tenant_id != params.company_id {
            return Err(LedgerError::Validation(
                "tenant/company mismatch for audit trail".to_string(),
            ));
        }

        let limit = params.limit;
        let request = build_audit_trail_filter(params, tenant);
        self.service.list_audit_trail(request).await.map(|events| {
            let has_more = limit
                .and_then(|limit| (events.len() == limit).then_some(()))
                .is_some();
            let next_cursor = if has_more {
                events.last().map(|event| event.id.clone())
            } else {
                None
            };
            build_audit_trail_response(events, next_cursor)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::LedgerAccount;
    use codex_app_server_protocol::LedgerAccountType;
    use codex_app_server_protocol::LedgerCurrency;
    use codex_app_server_protocol::LedgerCurrencyMode;
    use codex_app_server_protocol::LedgerEntryOrigin;
    use codex_app_server_protocol::LedgerEntryStatus;
    use codex_app_server_protocol::LedgerFiscalCalendar;
    use codex_app_server_protocol::LedgerJournalEntry;
    use codex_app_server_protocol::LedgerJournalLine;
    use codex_app_server_protocol::LedgerListAuditTrailParams;
    use codex_app_server_protocol::LedgerPeriodRef;
    use codex_app_server_protocol::LedgerPostingMode;
    use codex_app_server_protocol::LedgerPostingSide;
    use codex_app_server_protocol::LedgerReconciliationStatus;
    use codex_app_server_protocol::LedgerRevalueCurrencyParams;
    use codex_ledger::AccountType;
    use codex_ledger::ChartAccount;
    use codex_ledger::CurrencyMode as CoreCurrencyMode;
    use codex_ledger::EnsurePeriodRequest;
    use codex_ledger::InMemoryLedgerService;
    use codex_ledger::PeriodRef;
    use codex_ledger::PeriodState;
    use codex_ledger::Role as LedgerRole;
    use codex_ledger::SeedChartRequest;
    use pretty_assertions::assert_eq;
    use tokio::runtime::Runtime;

    fn demo_tenant(company_id: &str) -> LedgerTenantContext {
        LedgerTenantContext {
            tenant_id: company_id.to_string(),
            user_id: "test".into(),
            roles: vec![LedgerRole::Accountant],
            locale: Some("en-US".into()),
        }
    }

    #[test]
    fn facade_posts_entry() {
        let runtime = Runtime::new().expect("failed to create runtime");
        runtime.block_on(async {
            let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
            let facade = LedgerFacade::new(service.clone());

            let admin_tenant = LedgerTenantContext {
                tenant_id: "ledger-admin".into(),
                user_id: "test".into(),
                roles: vec![LedgerRole::ServiceAccount],
                locale: None,
            };

            let company = facade
                .create_company(
                    LedgerCreateCompanyParams {
                        name: "Demo Co".into(),
                        base_currency: LedgerCurrency {
                            code: "USD".into(),
                            precision: 2,
                        },
                        fiscal_calendar: LedgerFiscalCalendar {
                            periods_per_year: 12,
                            opening_month: 1,
                        },
                    },
                    admin_tenant.clone(),
                )
                .await
                .expect("company");

            let company_id = company.company.id.clone();
            let usd = LedgerCurrency {
                code: "USD".into(),
                precision: 2,
            };

            let cash_account = LedgerAccount {
                id: "cash".into(),
                company_id: company_id.clone(),
                code: "1000".into(),
                name: "Cash".into(),
                account_type: LedgerAccountType::Asset,
                parent_account_id: None,
                currency_mode: LedgerCurrencyMode::FunctionalOnly,
                tax_code: None,
                is_summary: false,
                is_active: true,
            };

            facade
                .upsert_account(
                    LedgerUpsertAccountParams {
                        account: cash_account.clone(),
                    },
                    admin_tenant.clone(),
                )
                .await
                .expect("account");

            let revenue_account = LedgerAccount {
                id: "revenue".into(),
                company_id: company_id.clone(),
                code: "4000".into(),
                name: "Revenue".into(),
                account_type: LedgerAccountType::Revenue,
                parent_account_id: None,
                currency_mode: LedgerCurrencyMode::FunctionalOnly,
                tax_code: None,
                is_summary: false,
                is_active: true,
            };

            facade
                .upsert_account(
                    LedgerUpsertAccountParams {
                        account: revenue_account.clone(),
                    },
                    admin_tenant.clone(),
                )
                .await
                .expect("account");

            let company_tenant = demo_tenant(&company_id);
            let response = facade
                .post_entry(
                    LedgerPostEntryParams {
                        entry: LedgerJournalEntry {
                            id: "sale-1".into(),
                            journal_id: "jnl-gl".into(),
                            status: LedgerEntryStatus::Draft,
                            reconciliation_status: LedgerReconciliationStatus::Unreconciled,
                            lines: vec![
                                LedgerJournalLine {
                                    id: "1".into(),
                                    account_id: cash_account.id.clone(),
                                    side: LedgerPostingSide::Debit,
                                    amount_minor: 15_000,
                                    currency: usd.clone(),
                                    functional_amount_minor: 15_000,
                                    functional_currency: usd.clone(),
                                    exchange_rate: None,
                                    tax_code: None,
                                    memo: Some("Cash sale".into()),
                                },
                                LedgerJournalLine {
                                    id: "2".into(),
                                    account_id: revenue_account.id.clone(),
                                    side: LedgerPostingSide::Credit,
                                    amount_minor: 15_000,
                                    currency: usd.clone(),
                                    functional_amount_minor: 15_000,
                                    functional_currency: usd.clone(),
                                    exchange_rate: None,
                                    tax_code: None,
                                    memo: Some("Cash sale".into()),
                                },
                            ],
                            origin: LedgerEntryOrigin::Manual,
                            memo: Some("Demo sale".into()),
                            reverses_entry_id: None,
                            reversed_by_entry_id: None,
                        },
                        mode: LedgerPostingMode::Commit,
                    },
                    company_tenant,
                )
                .await
                .expect("post entry");

            assert_eq!(response.entry.lines.len(), 2);
            assert_eq!(response.entry.status, LedgerEntryStatus::Posted);
        });
    }

    #[test]
    fn facade_lists_audit_trail_events() {
        let runtime = Runtime::new().expect("failed to create runtime");
        runtime.block_on(async {
            let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
            let facade = LedgerFacade::new(service.clone());

            let admin = LedgerTenantContext {
                tenant_id: "ledger-admin".into(),
                user_id: "svc".into(),
                roles: vec![LedgerRole::ServiceAccount],
                locale: None,
            };

            let company = facade
                .create_company(
                    LedgerCreateCompanyParams {
                        name: "Audit Co".into(),
                        base_currency: LedgerCurrency {
                            code: "USD".into(),
                            precision: 2,
                        },
                        fiscal_calendar: LedgerFiscalCalendar {
                            periods_per_year: 12,
                            opening_month: 1,
                        },
                    },
                    admin.clone(),
                )
                .await
                .expect("company");

            let company_id = company.company.id.clone();
            let tenant = demo_tenant(&company_id);

            facade
                .lock_period(
                    LedgerLockPeriodParams {
                        company_id: company_id.clone(),
                        journal_id: "jnl-gl".into(),
                        period: LedgerPeriodRef {
                            fiscal_year: 2025,
                            period: 1,
                        },
                        action: codex_app_server_protocol::LedgerPeriodAction::SoftClose,
                        approval_reference: None,
                    },
                    tenant.clone(),
                )
                .await
                .expect("lock");

            let response = facade
                .list_audit_trail(
                    LedgerListAuditTrailParams {
                        company_id: company_id.clone(),
                        entity_id: Some("jnl-gl".into()),
                        limit: Some(10),
                        cursor: None,
                    },
                    tenant,
                )
                .await
                .expect("audit");

            assert_eq!(response.events.len(), 1);
            assert_eq!(response.events[0].entity_id, "jnl-gl");
            assert_eq!(response.next_cursor, None);
        });
    }

    #[test]
    fn facade_revalues_currency_for_company() {
        let runtime = Runtime::new().expect("failed to create runtime");
        runtime.block_on(async {
            let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
            let facade = LedgerFacade::new(service.clone());

            let admin = LedgerTenantContext {
                tenant_id: "ledger-admin".into(),
                user_id: "svc".into(),
                roles: vec![LedgerRole::ServiceAccount],
                locale: None,
            };

            let company = facade
                .create_company(
                    LedgerCreateCompanyParams {
                        name: "FX Co".into(),
                        base_currency: LedgerCurrency {
                            code: "USD".into(),
                            precision: 2,
                        },
                        fiscal_calendar: LedgerFiscalCalendar {
                            periods_per_year: 12,
                            opening_month: 1,
                        },
                    },
                    admin.clone(),
                )
                .await
                .expect("company");

            let company_id = company.company.id.clone();
            let tenant = demo_tenant(&company_id);

            let response = facade
                .revalue_currency(
                    LedgerRevalueCurrencyParams {
                        company_id: company_id.clone(),
                        journal_id: "jnl-gl".into(),
                        period: LedgerPeriodRef {
                            fiscal_year: 2025,
                            period: 2,
                        },
                        currencies: vec![LedgerCurrency {
                            code: "EUR".into(),
                            precision: 2,
                        }],
                    },
                    tenant,
                )
                .await
                .expect("revalue");

            assert!(response.entries.is_empty());
            assert_eq!(response.next_cursor, None);
        });
    }

    #[test]
    fn facade_seeds_chart_and_ensures_period() {
        let runtime = Runtime::new().expect("failed to create runtime");
        runtime.block_on(async {
            let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
            let facade = LedgerFacade::new(service);

            let admin = demo_tenant("chart-admin");
            let company = facade
                .create_company(
                    LedgerCreateCompanyParams {
                        name: "Chart Co".into(),
                        base_currency: LedgerCurrency {
                            code: "USD".into(),
                            precision: 2,
                        },
                        fiscal_calendar: LedgerFiscalCalendar {
                            periods_per_year: 12,
                            opening_month: 1,
                        },
                    },
                    admin.clone(),
                )
                .await
                .expect("create company");

            let company_id = company.company.id.clone();
            let tenant = demo_tenant(&company_id);

            let accounts = facade
                .seed_chart(SeedChartRequest {
                    company_id: company_id.clone(),
                    tenant: tenant.clone(),
                    accounts: vec![
                        ChartAccount {
                            code: "1000".into(),
                            name: "Assets".into(),
                            account_type: AccountType::Asset,
                            parent_code: None,
                            currency_mode: CoreCurrencyMode::FunctionalOnly,
                            tax_code: None,
                            is_summary: true,
                        },
                        ChartAccount {
                            code: "1100".into(),
                            name: "Cash".into(),
                            account_type: AccountType::Asset,
                            parent_code: Some("1000".into()),
                            currency_mode: CoreCurrencyMode::FunctionalOnly,
                            tax_code: None,
                            is_summary: false,
                        },
                    ],
                })
                .await
                .expect("seed chart");

            assert_eq!(accounts.len(), 2);

            let journal = facade
                .ensure_period(EnsurePeriodRequest {
                    journal_id: "jnl-gl".into(),
                    period: PeriodRef {
                        fiscal_year: 2026,
                        period: 2,
                    },
                    tenant,
                })
                .await
                .expect("ensure period");

            assert_eq!(journal.period_state, PeriodState::Open);
        });
    }
}
