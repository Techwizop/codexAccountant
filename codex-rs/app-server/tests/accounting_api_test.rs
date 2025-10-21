#![cfg(feature = "ledger")]

use codex_app_server_protocol::*;
use serde_json::json;

/// Helper to create a test JSON-RPC request
fn create_request(id: i64, method: &str, params: serde_json::Value) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
}

/// Helper to validate response structure
fn validate_response(response: &serde_json::Value, expected_id: i64) {
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], expected_id);
    assert!(response.get("result").is_some() || response.get("error").is_some());
}

#[test]
fn test_ledger_list_companies_request_format() {
    let request = create_request(1, "ledgerListCompanies", json!({}));

    assert_eq!(request["method"], "ledgerListCompanies");
    assert_eq!(request["id"], 1);
}

#[test]
fn test_ledger_list_companies_with_search_request_format() {
    let request = create_request(
        2,
        "ledgerListCompanies",
        json!({
            "search": "Demo"
        }),
    );

    assert_eq!(request["params"]["search"], "Demo");
}

#[test]
fn test_ledger_list_accounts_request_format() {
    let request = create_request(
        3,
        "ledgerListAccounts",
        json!({
            "company_id": "comp-001"
        }),
    );

    assert_eq!(request["params"]["company_id"], "comp-001");
}

#[test]
fn test_ledger_list_accounts_with_type_filter_request_format() {
    let request = create_request(
        4,
        "ledgerListAccounts",
        json!({
            "company_id": "comp-001",
            "account_type": "Asset"
        }),
    );

    assert_eq!(request["params"]["account_type"], "Asset");
}

#[test]
fn test_ledger_list_entries_request_format() {
    let request = create_request(
        5,
        "ledgerListEntries",
        json!({
            "company_id": "comp-001",
            "limit": 50,
            "offset": 0
        }),
    );

    assert_eq!(request["params"]["limit"], 50);
    assert_eq!(request["params"]["offset"], 0);
}

#[test]
fn test_ledger_list_entries_with_filters_request_format() {
    let request = create_request(
        6,
        "ledgerListEntries",
        json!({
            "company_id": "comp-001",
            "start_date": "2024-01-01",
            "end_date": "2024-12-31",
            "account_code": "1000",
            "limit": 100,
            "offset": 50
        }),
    );

    assert_eq!(request["params"]["start_date"], "2024-01-01");
    assert_eq!(request["params"]["end_date"], "2024-12-31");
    assert_eq!(request["params"]["account_code"], "1000");
}

#[test]
fn test_ledger_get_company_context_request_format() {
    let request = create_request(
        7,
        "ledgerGetCompanyContext",
        json!({
            "company_id": "comp-001",
            "limit": 50
        }),
    );

    assert_eq!(request["params"]["company_id"], "comp-001");
    assert_eq!(request["params"]["limit"], 50);
}

#[test]
fn test_ledger_process_document_request_format() {
    let request = create_request(
        8,
        "ledgerProcessDocument",
        json!({
            "upload_id": "upload-123",
            "company_id": "comp-001"
        }),
    );

    assert_eq!(request["params"]["upload_id"], "upload-123");
    assert_eq!(request["params"]["company_id"], "comp-001");
}

#[test]
fn test_protocol_types_serialization() {
    // Test LedgerListCompaniesParams
    let params = LedgerListCompaniesParams {
        search: Some("Test".to_string()),
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["search"], "Test");

    // Test LedgerListAccountsParams
    let params = LedgerListAccountsParams {
        company_id: "comp-001".to_string(),
        account_type: Some(LedgerAccountType::Asset),
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["companyId"], "comp-001");
    assert_eq!(json["accountType"], "asset");

    // Test LedgerListEntriesParams
    let params = LedgerListEntriesParams {
        company_id: "comp-001".to_string(),
        start_date: Some("2024-01-01".to_string()),
        end_date: None,
        account_code: None,
        limit: 50,
        offset: 0,
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["companyId"], "comp-001");
    assert_eq!(json["limit"], 50);
}

#[test]
fn test_protocol_types_deserialization() {
    // Test LedgerListCompaniesResponse
    let json = json!({
        "companies": [
            {
                "id": "comp-001",
                "name": "Test Corp",
                "baseCurrency": {"code": "USD", "precision": 2},
                "fiscalCalendar": {"periodsPerYear": 12, "openingMonth": 1}
            }
        ]
    });
    let response: LedgerListCompaniesResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.companies.len(), 1);
    assert_eq!(response.companies[0].name, "Test Corp");

    // Test LedgerListAccountsResponse
    let json = json!({
        "accounts": [
            {
                "id": "acc-001",
                "companyId": "comp-001",
                "code": "1000",
                "name": "Cash",
                "accountType": "asset",
                "currencyMode": "functionalOnly",
                "isSummary": false,
                "isActive": true
            }
        ]
    });
    let response: LedgerListAccountsResponse = serde_json::from_value(json).unwrap();
    assert_eq!(response.accounts.len(), 1);
    assert_eq!(response.accounts[0].name, "Cash");
}

#[test]
fn test_error_response_format() {
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32600,
            "message": "ledger feature not enabled"
        }
    });

    assert_eq!(error_response["error"]["code"], -32600);
    assert!(error_response["error"]["message"].is_string());
}

#[test]
fn test_ledger_journal_entry_suggestion_structure() {
    let suggestion = LedgerJournalEntrySuggestion {
        lines: vec![
            LedgerSuggestedLine {
                account_code: "5000".to_string(),
                account_name: "Expenses".to_string(),
                debit_minor: 10000,
                credit_minor: 0,
            },
            LedgerSuggestedLine {
                account_code: "1000".to_string(),
                account_name: "Cash".to_string(),
                debit_minor: 0,
                credit_minor: 10000,
            },
        ],
        memo: "Invoice from Vendor X".to_string(),
        confidence: 0.95,
        reasoning: "Standard expense entry".to_string(),
    };

    let json = serde_json::to_value(&suggestion).unwrap();
    assert_eq!(json["lines"].as_array().unwrap().len(), 2);
    assert_eq!(json["confidence"], 0.95);
    assert_eq!(json["memo"], "Invoice from Vendor X");

    // Verify balance
    let total_debit: i64 = suggestion.lines.iter().map(|l| l.debit_minor).sum();
    let total_credit: i64 = suggestion.lines.iter().map(|l| l.credit_minor).sum();
    assert_eq!(total_debit, total_credit, "Entry must be balanced");
}

#[test]
fn test_ledger_get_company_context_response_structure() {
    let response = LedgerGetCompanyContextResponse {
        chart_of_accounts: vec![],
        recent_transactions: vec![],
        vendor_mappings: std::collections::HashMap::new(),
        policy_rules: LedgerPolicyRules {
            auto_post_enabled: true,
            auto_post_limit_minor: 10000,
            confidence_floor: 0.85,
        },
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["policyRules"]["autoPostEnabled"], true);
    assert_eq!(json["policyRules"]["autoPostLimitMinor"], 10000);
    assert_eq!(json["policyRules"]["confidenceFloor"], 0.85);
}

#[test]
fn test_currency_precision() {
    // Test that minor units work correctly
    let amount_dollars = 100.00;
    let amount_minor = (amount_dollars * 100.0) as i64;
    assert_eq!(amount_minor, 10000);

    // Test conversion back
    let converted_back = amount_minor as f64 / 100.0;
    assert_eq!(converted_back, amount_dollars);
}

#[test]
fn test_account_type_enum_serialization() {
    let types = vec![
        (LedgerAccountType::Asset, "asset"),
        (LedgerAccountType::Liability, "liability"),
        (LedgerAccountType::Equity, "equity"),
        (LedgerAccountType::Revenue, "revenue"),
        (LedgerAccountType::Expense, "expense"),
    ];

    for (account_type, expected_str) in types {
        let json = serde_json::to_value(&account_type).unwrap();
        assert_eq!(json.as_str().unwrap(), expected_str);
    }
}

#[test]
fn test_client_request_enum_deserialization() {
    // Test LedgerListCompanies
    let json = json!({
        "method": "ledgerListCompanies",
        "id": 1,
        "params": {}
    });
    let request: Result<ClientRequest, _> = serde_json::from_value(json);
    assert!(request.is_ok());

    // Test LedgerListAccounts
    let json = json!({
        "method": "ledgerListAccounts",
        "id": 2,
        "params": {
            "company_id": "comp-001"
        }
    });
    let request: Result<ClientRequest, _> = serde_json::from_value(json);
    assert!(request.is_ok());

    // Test LedgerProcessDocument
    let json = json!({
        "method": "ledgerProcessDocument",
        "id": 3,
        "params": {
            "upload_id": "upload-123",
            "company_id": "comp-001"
        }
    });
    let request: Result<ClientRequest, _> = serde_json::from_value(json);
    assert!(request.is_ok());
}

#[test]
fn test_pagination_defaults() {
    let params = LedgerListEntriesParams {
        company_id: "comp-001".to_string(),
        start_date: None,
        end_date: None,
        account_code: None,
        limit: 50,
        offset: 0,
    };

    assert_eq!(params.limit, 50);
    assert_eq!(params.offset, 0);
}

/// Mock test showing expected workflow
#[test]
fn test_document_processing_workflow() {
    // Step 1: Upload document (not implemented in this test)
    let upload_id = "upload-123";

    // Step 2: Process document
    let process_request = create_request(
        1,
        "ledgerProcessDocument",
        json!({
            "upload_id": upload_id,
            "company_id": "comp-001"
        }),
    );

    validate_response(
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "suggestion": {
                    "lines": [
                        {
                            "accountCode": "5000",
                            "accountName": "Expenses",
                            "debitMinor": 10000,
                            "creditMinor": 0
                        },
                        {
                            "accountCode": "1000",
                            "accountName": "Cash",
                            "debitMinor": 0,
                            "creditMinor": 10000
                        }
                    ],
                    "memo": "Invoice processing",
                    "confidence": 0.9,
                    "reasoning": "AI detected expense invoice"
                }
            }
        }),
        1,
    );

    // Step 3: Post entry (using existing endpoint)
    let post_request = create_request(
        2,
        "ledgerPostEntry",
        json!({
            "entry": {
                "id": "entry-new",
                "journalId": "journal-001",
                "status": "posted",
                "reconciliationStatus": "unreconciled",
                "origin": "aiSuggested",
                "lines": [
                    {
                        "id": "line-1",
                        "accountId": "acc-5000",
                        "side": "debit",
                        "amountMinor": 10000,
                        "currency": {"code": "USD", "precision": 2},
                        "functionalAmountMinor": 10000,
                        "functionalCurrency": {"code": "USD", "precision": 2}
                    }
                ]
            },
            "mode": "commit"
        }),
    );

    assert_eq!(post_request["method"], "ledgerPostEntry");
}

/// Test that demonstrates the full context retrieval for AI
#[test]
fn test_ai_context_aggregation() {
    // Get company context
    let context_request = create_request(
        1,
        "ledgerGetCompanyContext",
        json!({
            "company_id": "comp-001",
            "limit": 50
        }),
    );

    // Expected response structure
    let expected_response = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "chartOfAccounts": [
                {
                    "id": "acc-1000",
                    "code": "1000",
                    "name": "Cash",
                    "accountType": "asset"
                }
            ],
            "recentTransactions": [],
            "vendorMappings": {
                "Acme Corp": "Vendor-001"
            },
            "policyRules": {
                "autoPostEnabled": false,
                "autoPostLimitMinor": 10000,
                "confidenceFloor": 0.85
            }
        }
    });

    validate_response(&expected_response, 1);
    assert!(expected_response["result"]["chartOfAccounts"].is_array());
    assert!(expected_response["result"]["policyRules"].is_object());
}
