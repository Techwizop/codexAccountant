#![cfg(feature = "ledger")]

use std::collections::HashMap;
use std::sync::Arc;

use codex_accounting_api::LedgerFacade;
use codex_app_server_protocol::*;
use codex_core::accounting::{DocumentAgent, JournalEntrySuggestion};

pub struct AccountingHandlers {
    ledger_facade: Arc<LedgerFacade>,
    document_agent: Arc<DocumentAgent>,
}

impl AccountingHandlers {
    pub fn new(ledger_facade: Arc<LedgerFacade>, document_agent: Arc<DocumentAgent>) -> Self {
        Self {
            ledger_facade,
            document_agent,
        }
    }

    pub async fn list_companies(
        &self,
        params: LedgerListCompaniesParams,
    ) -> Result<LedgerListCompaniesResponse, String> {
        // TODO: Call actual ledger facade method when available
        // For now, return mock data

        let companies = vec![LedgerCompany {
            id: "comp-001".to_string(),
            name: "Demo Corporation".to_string(),
            base_currency: LedgerCurrency {
                code: "USD".to_string(),
                precision: 2,
            },
            fiscal_calendar: LedgerFiscalCalendar {
                periods_per_year: 12,
                opening_month: 1,
            },
            metadata: None,
        }];

        // Filter by search if provided
        let filtered = if let Some(search) = &params.search {
            companies
                .into_iter()
                .filter(|c| c.name.to_lowercase().contains(&search.to_lowercase()))
                .collect()
        } else {
            companies
        };

        Ok(LedgerListCompaniesResponse {
            companies: filtered,
        })
    }

    pub async fn list_accounts(
        &self,
        params: LedgerListAccountsParams,
    ) -> Result<LedgerListAccountsResponse, String> {
        // TODO: Call actual ledger facade method
        // For now, return mock chart of accounts

        let accounts = vec![
            LedgerAccount {
                id: format!("acc-{}-1000", params.company_id),
                company_id: params.company_id.clone(),
                code: "1000".to_string(),
                name: "Cash".to_string(),
                account_type: LedgerAccountType::Asset,
                parent_account_id: None,
                currency_mode: LedgerCurrencyMode::FunctionalOnly,
                tax_code: None,
                is_summary: false,
                is_active: true,
            },
            LedgerAccount {
                id: format!("acc-{}-2000", params.company_id),
                company_id: params.company_id.clone(),
                code: "2000".to_string(),
                name: "Accounts Payable".to_string(),
                account_type: LedgerAccountType::Liability,
                parent_account_id: None,
                currency_mode: LedgerCurrencyMode::FunctionalOnly,
                tax_code: None,
                is_summary: false,
                is_active: true,
            },
            LedgerAccount {
                id: format!("acc-{}-5000", params.company_id),
                company_id: params.company_id.clone(),
                code: "5000".to_string(),
                name: "Operating Expenses".to_string(),
                account_type: LedgerAccountType::Expense,
                parent_account_id: None,
                currency_mode: LedgerCurrencyMode::FunctionalOnly,
                tax_code: None,
                is_summary: false,
                is_active: true,
            },
        ];

        // Filter by account type if provided
        let filtered = if let Some(account_type) = params.account_type {
            accounts
                .into_iter()
                .filter(|a| a.account_type == account_type)
                .collect()
        } else {
            accounts
        };

        Ok(LedgerListAccountsResponse { accounts: filtered })
    }

    pub async fn list_entries(
        &self,
        params: LedgerListEntriesParams,
    ) -> Result<LedgerListEntriesResponse, String> {
        // TODO: Call actual ledger facade method
        // For now, return empty list

        Ok(LedgerListEntriesResponse {
            entries: vec![],
            total_count: 0,
        })
    }

    pub async fn get_company_context(
        &self,
        params: LedgerGetCompanyContextParams,
    ) -> Result<LedgerGetCompanyContextResponse, String> {
        // Get chart of accounts
        let accounts_response = self
            .list_accounts(LedgerListAccountsParams {
                company_id: params.company_id.clone(),
                account_type: None,
            })
            .await?;

        // Get recent transactions
        let entries_response = self
            .list_entries(LedgerListEntriesParams {
                company_id: params.company_id.clone(),
                start_date: None,
                end_date: None,
                account_code: None,
                limit: params.limit,
                offset: 0,
            })
            .await?;

        // Build vendor mappings (TODO: get from actual storage)
        let vendor_mappings = HashMap::new();

        // Build policy rules (TODO: get from policy engine)
        let policy_rules = LedgerPolicyRules {
            auto_post_enabled: false,
            auto_post_limit_minor: 10000, // $100.00
            confidence_floor: 0.85,
        };

        Ok(LedgerGetCompanyContextResponse {
            chart_of_accounts: accounts_response.accounts,
            recent_transactions: entries_response.entries,
            vendor_mappings,
            policy_rules,
        })
    }

    pub async fn process_document(
        &self,
        params: LedgerProcessDocumentParams,
    ) -> Result<LedgerProcessDocumentResponse, String> {
        // Call document agent from Phase 1
        let suggestion = self
            .document_agent
            .process_document(&params.upload_id, &params.company_id)
            .await
            .map_err(|e| format!("Document processing failed: {e}"))?;

        // Convert from core types to protocol types
        let protocol_suggestion = convert_suggestion_to_protocol(suggestion);

        Ok(LedgerProcessDocumentResponse {
            suggestion: protocol_suggestion,
        })
    }
}

/// Convert from core JournalEntrySuggestion to protocol LedgerJournalEntrySuggestion
fn convert_suggestion_to_protocol(
    suggestion: JournalEntrySuggestion,
) -> LedgerJournalEntrySuggestion {
    let lines = suggestion
        .lines
        .into_iter()
        .map(|line| LedgerSuggestedLine {
            account_code: line.account_code,
            account_name: line.account_name,
            debit_minor: line.debit_minor,
            credit_minor: line.credit_minor,
        })
        .collect();

    LedgerJournalEntrySuggestion {
        lines,
        memo: suggestion.memo,
        confidence: suggestion.confidence,
        reasoning: suggestion.reasoning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_ledger::InMemoryLedgerService;

    fn setup_handlers() -> AccountingHandlers {
        let service = Arc::new(InMemoryLedgerService::new());
        let ledger_facade = Arc::new(LedgerFacade::new(service));
        let document_agent = Arc::new(DocumentAgent::new());

        AccountingHandlers::new(ledger_facade, document_agent)
    }

    #[tokio::test]
    async fn test_list_companies() {
        let handlers = setup_handlers();

        let params = LedgerListCompaniesParams { search: None };
        let result = handlers.list_companies(params).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.companies.is_empty());
    }

    #[tokio::test]
    async fn test_list_companies_with_search() {
        let handlers = setup_handlers();

        let params = LedgerListCompaniesParams {
            search: Some("Demo".to_string()),
        };
        let result = handlers.list_companies(params).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.companies.len(), 1);
        assert_eq!(response.companies[0].name, "Demo Corporation");
    }

    #[tokio::test]
    async fn test_list_accounts() {
        let handlers = setup_handlers();

        let params = LedgerListAccountsParams {
            company_id: "comp-001".to_string(),
            account_type: None,
        };
        let result = handlers.list_accounts(params).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.accounts.len(), 3);
    }

    #[tokio::test]
    async fn test_list_accounts_filtered_by_type() {
        let handlers = setup_handlers();

        let params = LedgerListAccountsParams {
            company_id: "comp-001".to_string(),
            account_type: Some(LedgerAccountType::Asset),
        };
        let result = handlers.list_accounts(params).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.accounts.len(), 1);
        assert_eq!(response.accounts[0].account_type, LedgerAccountType::Asset);
    }

    #[tokio::test]
    async fn test_get_company_context() {
        let handlers = setup_handlers();

        let params = LedgerGetCompanyContextParams {
            company_id: "comp-001".to_string(),
            limit: 50,
        };
        let result = handlers.get_company_context(params).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.chart_of_accounts.is_empty());
        assert_eq!(response.policy_rules.confidence_floor, 0.85);
    }
}
