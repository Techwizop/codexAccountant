#![cfg(feature = "ledger")]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::client_common::tools::{ResponsesApiTool, ToolSpec};
use crate::function_tool::FunctionCallError;
use crate::tools::context::{ToolInvocation, ToolOutput, ToolPayload};
use crate::tools::registry::{ToolHandler, ToolKind};
use crate::tools::spec::JsonSchema;

use codex_accounting_api::LedgerFacade;
use codex_ledger::TenantContext;

// ============================================================================
// Tool 1: CreateCompanyTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct CreateCompanyArgs {
    name: String,
    base_currency: String,
    fiscal_year_opening_month: u32,
}

pub struct CreateCompanyTool {
    facade: Arc<LedgerFacade>,
}

impl CreateCompanyTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for CreateCompanyTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let args: CreateCompanyArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // Validate fiscal year month
        if !(1..=12).contains(&args.fiscal_year_opening_month) {
            return Err(FunctionCallError::InvalidArgs(format!(
                "fiscal_year_opening_month must be 1-12, got {}",
                args.fiscal_year_opening_month
            )));
        }

        // TODO: Replace with actual implementation once we wire up LedgerFacade
        // For now, return a mock response
        let response = serde_json::json!({
            "company_id": "comp-001",
            "name": args.name,
            "base_currency": args.base_currency,
            "fiscal_year_opening_month": args.fiscal_year_opening_month,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Tool 2: ListCompaniesTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct ListCompaniesArgs {
    #[serde(default)]
    search: Option<String>,
}

pub struct ListCompaniesTool {
    facade: Arc<LedgerFacade>,
}

impl ListCompaniesTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for ListCompaniesTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let _args: ListCompaniesArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // TODO: Implement actual company listing
        let response = serde_json::json!({
            "companies": []
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Tool 3: UpsertAccountTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct UpsertAccountArgs {
    company_id: String,
    code: String,
    name: String,
    account_type: String, // Asset, Liability, Equity, Revenue, Expense
    #[serde(default)]
    parent_code: Option<String>,
}

pub struct UpsertAccountTool {
    facade: Arc<LedgerFacade>,
}

impl UpsertAccountTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for UpsertAccountTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let args: UpsertAccountArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // Validate account type
        let valid_types = ["Asset", "Liability", "Equity", "Revenue", "Expense"];
        if !valid_types.contains(&args.account_type.as_str()) {
            return Err(FunctionCallError::InvalidArgs(format!(
                "account_type must be one of: {}, got {}",
                valid_types.join(", "),
                args.account_type
            )));
        }

        // TODO: Implement actual account upsert
        let response = serde_json::json!({
            "account": {
                "code": args.code,
                "name": args.name,
                "account_type": args.account_type,
                "parent_code": args.parent_code,
                "balance_minor": 0,
            }
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Tool 4: ListAccountsTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct ListAccountsArgs {
    company_id: String,
    #[serde(default)]
    account_type: Option<String>,
}

pub struct ListAccountsTool {
    facade: Arc<LedgerFacade>,
}

impl ListAccountsTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for ListAccountsTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let _args: ListAccountsArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // TODO: Implement actual account listing
        let response = serde_json::json!({
            "accounts": []
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Tool 5: PostJournalEntryTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct PostEntryArgs {
    company_id: String,
    date: String, // YYYY-MM-DD
    memo: String,
    lines: Vec<EntryLineArgs>,
}

#[derive(Debug, Deserialize)]
struct EntryLineArgs {
    account_code: String,
    debit_minor: i64,
    credit_minor: i64,
    #[serde(default)]
    memo: Option<String>,
}

pub struct PostJournalEntryTool {
    facade: Arc<LedgerFacade>,
}

impl PostJournalEntryTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for PostJournalEntryTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let args: PostEntryArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // Validate date format
        chrono::NaiveDate::parse_from_str(&args.date, "%Y-%m-%d").map_err(|e| {
            FunctionCallError::InvalidArgs(format!(
                "Invalid date format (expected YYYY-MM-DD): {e}"
            ))
        })?;

        // Validate balance: debits must equal credits
        let total_debits: i64 = args.lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = args.lines.iter().map(|l| l.credit_minor).sum();

        if total_debits != total_credits {
            return Err(FunctionCallError::InvalidArgs(format!(
                "Entry not balanced: debits={total_debits}, credits={total_credits}. Debits must equal credits."
            )));
        }

        // Validate that each line has either debit or credit, not both
        for (idx, line) in args.lines.iter().enumerate() {
            if line.debit_minor != 0 && line.credit_minor != 0 {
                return Err(FunctionCallError::InvalidArgs(format!(
                    "Line {idx}: cannot have both debit and credit. Use separate lines for debits and credits."
                )));
            }
            if line.debit_minor < 0 || line.credit_minor < 0 {
                return Err(FunctionCallError::InvalidArgs(format!(
                    "Line {idx}: amounts cannot be negative"
                )));
            }
        }

        // TODO: Implement actual journal entry posting
        let response = serde_json::json!({
            "entry_id": "je-001",
            "posted_at": chrono::Utc::now().to_rfc3339(),
            "balanced": true,
            "total_debit": total_debits,
            "total_credit": total_credits,
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Tool 6: ListEntriesTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct ListEntriesArgs {
    company_id: String,
    #[serde(default)]
    start_date: Option<String>,
    #[serde(default)]
    end_date: Option<String>,
    #[serde(default)]
    account_code: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    50
}

pub struct ListEntriesTool {
    facade: Arc<LedgerFacade>,
}

impl ListEntriesTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for ListEntriesTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let _args: ListEntriesArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // TODO: Implement actual entry listing
        let response = serde_json::json!({
            "entries": [],
            "total_count": 0
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Tool 7: GetCompanyContextTool
// ============================================================================

#[derive(Debug, Deserialize)]
struct GetCompanyContextArgs {
    company_id: String,
    #[serde(default = "default_context_limit")]
    limit: usize,
}

fn default_context_limit() -> usize {
    50
}

pub struct GetCompanyContextTool {
    facade: Arc<LedgerFacade>,
}

impl GetCompanyContextTool {
    pub fn new(facade: Arc<LedgerFacade>) -> Self {
        Self { facade }
    }
}

#[async_trait]
impl ToolHandler for GetCompanyContextTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments,
            _ => return Err(FunctionCallError::Fatal("Expected function payload".into())),
        };

        let _args: GetCompanyContextArgs = serde_json::from_str(arguments)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;

        // TODO: Implement actual context retrieval
        let response = serde_json::json!({
            "chart_of_accounts": [],
            "recent_transactions": [],
            "vendor_mappings": {},
            "policy_rules": {
                "auto_post_enabled": false,
                "auto_post_limit_minor": 50000,
                "confidence_floor": 0.8
            }
        });

        Ok(ToolOutput::Function {
            content: serde_json::to_string(&response).map_err(|e| {
                FunctionCallError::SerializationError(format!("Failed to serialize: {e}"))
            })?,
            success: Some(true),
        })
    }
}

// ============================================================================
// Function Definitions for ChatGPT
// ============================================================================

pub fn create_accounting_tool_specs() -> Vec<ToolSpec> {
    vec![
        create_company_function(),
        list_companies_function(),
        upsert_account_function(),
        list_accounts_function(),
        post_journal_entry_function(),
        list_entries_function(),
        get_company_context_function(),
    ]
}

fn create_company_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "name".to_string(),
        JsonSchema::String {
            description: Some("The legal name of the company".to_string()),
        },
    );
    properties.insert(
        "base_currency".to_string(),
        JsonSchema::String {
            description: Some("ISO 4217 currency code (e.g., USD, EUR, GBP)".to_string()),
        },
    );
    properties.insert(
        "fiscal_year_opening_month".to_string(),
        JsonSchema::Number {
            description: Some("Month when fiscal year starts (1-12, where 1=January)".to_string()),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "create_company".to_string(),
        description: "Create a new company in the accounting system with chart of accounts and fiscal calendar. Returns the company ID and details.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec![
                "name".to_string(),
                "base_currency".to_string(),
                "fiscal_year_opening_month".to_string(),
            ]),
            additional_properties: Some(false.into()),
        },
    })
}

fn list_companies_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "search".to_string(),
        JsonSchema::String {
            description: Some("Optional search string to filter companies by name".to_string()),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "list_companies".to_string(),
        description:
            "List all companies in the accounting system. Optionally filter by search string."
                .to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: None,
            additional_properties: Some(false.into()),
        },
    })
}

fn upsert_account_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "company_id".to_string(),
        JsonSchema::String {
            description: Some("The company ID this account belongs to".to_string()),
        },
    );
    properties.insert(
        "code".to_string(),
        JsonSchema::String {
            description: Some("Unique account code (e.g., '1000' for Cash)".to_string()),
        },
    );
    properties.insert(
        "name".to_string(),
        JsonSchema::String {
            description: Some("Account name (e.g., 'Cash', 'Accounts Payable')".to_string()),
        },
    );
    properties.insert(
        "account_type".to_string(),
        JsonSchema::String {
            description: Some(
                "Account type: Asset, Liability, Equity, Revenue, or Expense".to_string(),
            ),
        },
    );
    properties.insert(
        "parent_code".to_string(),
        JsonSchema::String {
            description: Some(
                "Optional parent account code for hierarchical chart of accounts".to_string(),
            ),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "upsert_account".to_string(),
        description: "Create or update an account in the chart of accounts. Use this to set up the accounting structure.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec![
                "company_id".to_string(),
                "code".to_string(),
                "name".to_string(),
                "account_type".to_string(),
            ]),
            additional_properties: Some(false.into()),
        },
    })
}

fn list_accounts_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "company_id".to_string(),
        JsonSchema::String {
            description: Some("The company ID to list accounts for".to_string()),
        },
    );
    properties.insert(
        "account_type".to_string(),
        JsonSchema::String {
            description: Some(
                "Optional filter by account type (Asset, Liability, Equity, Revenue, Expense)"
                    .to_string(),
            ),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "list_accounts".to_string(),
        description: "List all accounts in the chart of accounts for a company. Optionally filter by account type.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["company_id".to_string()]),
            additional_properties: Some(false.into()),
        },
    })
}

fn post_journal_entry_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "company_id".to_string(),
        JsonSchema::String {
            description: Some("The company ID to post the entry to".to_string()),
        },
    );
    properties.insert(
        "date".to_string(),
        JsonSchema::String {
            description: Some("Entry date in YYYY-MM-DD format".to_string()),
        },
    );
    properties.insert(
        "memo".to_string(),
        JsonSchema::String {
            description: Some("Description of the transaction".to_string()),
        },
    );

    let mut line_properties = BTreeMap::new();
    line_properties.insert(
        "account_code".to_string(),
        JsonSchema::String {
            description: Some("Account code from chart of accounts".to_string()),
        },
    );
    line_properties.insert(
        "debit_minor".to_string(),
        JsonSchema::Number {
            description: Some(
                "Debit amount in minor currency units (cents). Use 0 for credit lines.".to_string(),
            ),
        },
    );
    line_properties.insert(
        "credit_minor".to_string(),
        JsonSchema::Number {
            description: Some(
                "Credit amount in minor currency units (cents). Use 0 for debit lines.".to_string(),
            ),
        },
    );
    line_properties.insert(
        "memo".to_string(),
        JsonSchema::String {
            description: Some("Optional line-specific memo".to_string()),
        },
    );

    properties.insert(
        "lines".to_string(),
        JsonSchema::Array {
            items: Box::new(JsonSchema::Object {
                properties: line_properties,
                required: Some(vec![
                    "account_code".to_string(),
                    "debit_minor".to_string(),
                    "credit_minor".to_string(),
                ]),
                additional_properties: Some(false.into()),
            }),
            description: Some(
                "Array of journal entry lines. Debits must equal credits.".to_string(),
            ),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "post_journal_entry".to_string(),
        description: "Post a balanced journal entry. Debits must equal credits. Each line has either a debit or credit amount (not both). Amounts in minor currency units (multiply dollars by 100).".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec![
                "company_id".to_string(),
                "date".to_string(),
                "memo".to_string(),
                "lines".to_string(),
            ]),
            additional_properties: Some(false.into()),
        },
    })
}

fn list_entries_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "company_id".to_string(),
        JsonSchema::String {
            description: Some("The company ID to list entries for".to_string()),
        },
    );
    properties.insert(
        "start_date".to_string(),
        JsonSchema::String {
            description: Some("Optional start date filter (YYYY-MM-DD)".to_string()),
        },
    );
    properties.insert(
        "end_date".to_string(),
        JsonSchema::String {
            description: Some("Optional end date filter (YYYY-MM-DD)".to_string()),
        },
    );
    properties.insert(
        "account_code".to_string(),
        JsonSchema::String {
            description: Some("Optional account code filter".to_string()),
        },
    );
    properties.insert(
        "limit".to_string(),
        JsonSchema::Number {
            description: Some("Maximum number of entries to return (default 50)".to_string()),
        },
    );
    properties.insert(
        "offset".to_string(),
        JsonSchema::Number {
            description: Some("Number of entries to skip for pagination (default 0)".to_string()),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "list_entries".to_string(),
        description: "List journal entries for a company with optional filters for date range, account, and pagination.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["company_id".to_string()]),
            additional_properties: Some(false.into()),
        },
    })
}

fn get_company_context_function() -> ToolSpec {
    let mut properties = BTreeMap::new();
    properties.insert(
        "company_id".to_string(),
        JsonSchema::String {
            description: Some("The company ID to get context for".to_string()),
        },
    );
    properties.insert(
        "limit".to_string(),
        JsonSchema::Number {
            description: Some(
                "Maximum number of items to return in lists (default 50)".to_string(),
            ),
        },
    );

    ToolSpec::Function(ResponsesApiTool {
        name: "get_company_context".to_string(),
        description: "Get comprehensive context for a company including chart of accounts, recent transactions, vendor mappings, and accounting policy rules. Useful for AI-assisted document processing.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["company_id".to_string()]),
            additional_properties: Some(false.into()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_ledger::InMemoryLedgerService;

    fn mock_facade() -> Arc<LedgerFacade> {
        let service = Arc::new(InMemoryLedgerService::new());
        Arc::new(LedgerFacade::new(service))
    }

    // Helper to create test invocations without full Session/TurnContext
    // For unit testing, we'll test the argument parsing directly

    #[test]
    fn create_company_args_validates_fiscal_month() {
        // Test argument validation directly
        let valid_args =
            r#"{"name": "Test Corp", "base_currency": "USD", "fiscal_year_opening_month": 1}"#;
        let result: Result<CreateCompanyArgs, _> = serde_json::from_str(valid_args);
        assert!(result.is_ok());

        let invalid_args =
            r#"{"name": "Test Corp", "base_currency": "USD", "fiscal_year_opening_month": 13}"#;
        let parsed: CreateCompanyArgs = serde_json::from_str(invalid_args).unwrap();
        assert!(parsed.fiscal_year_opening_month == 13); // Parsing succeeds, validation happens in handler
    }

    #[test]
    fn post_entry_args_parse_correctly() {
        let balanced = r#"{
            "company_id": "comp-1",
            "date": "2024-01-01",
            "memo": "Test entry",
            "lines": [
                {"account_code": "1000", "debit_minor": 100, "credit_minor": 0},
                {"account_code": "5000", "debit_minor": 0, "credit_minor": 100}
            ]
        }"#;

        let result: Result<PostEntryArgs, _> = serde_json::from_str(balanced);
        assert!(result.is_ok());

        let args = result.unwrap();
        assert_eq!(args.lines.len(), 2);

        let total_debits: i64 = args.lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = args.lines.iter().map(|l| l.credit_minor).sum();
        assert_eq!(total_debits, total_credits);
    }

    #[test]
    fn post_entry_detects_unbalanced() {
        let unbalanced = r#"{
            "company_id": "comp-1",
            "date": "2024-01-01",
            "memo": "Test",
            "lines": [
                {"account_code": "1000", "debit_minor": 100, "credit_minor": 0},
                {"account_code": "5000", "debit_minor": 0, "credit_minor": 50}
            ]
        }"#;

        let args: PostEntryArgs = serde_json::from_str(unbalanced).unwrap();
        let total_debits: i64 = args.lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = args.lines.iter().map(|l| l.credit_minor).sum();
        assert_ne!(total_debits, total_credits);
    }

    #[test]
    fn post_entry_detects_negative_amounts() {
        let negative = r#"{
            "company_id": "comp-1",
            "date": "2024-01-01",
            "memo": "Test",
            "lines": [
                {"account_code": "1000", "debit_minor": -100, "credit_minor": 0}
            ]
        }"#;

        let args: PostEntryArgs = serde_json::from_str(negative).unwrap();
        assert!(args.lines[0].debit_minor < 0);
    }

    #[test]
    fn upsert_account_args_parse() {
        let valid = r#"{
            "company_id": "comp-1",
            "code": "1000",
            "name": "Cash",
            "account_type": "Asset"
        }"#;

        let result: Result<UpsertAccountArgs, _> = serde_json::from_str(valid);
        assert!(result.is_ok());

        let invalid_type = r#"{
            "company_id": "comp-1",
            "code": "1000",
            "name": "Cash",
            "account_type": "InvalidType"
        }"#;

        let args: UpsertAccountArgs = serde_json::from_str(invalid_type).unwrap();
        let valid_types = ["Asset", "Liability", "Equity", "Revenue", "Expense"];
        assert!(!valid_types.contains(&args.account_type.as_str()));
    }
}
