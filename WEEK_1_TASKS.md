# Week 1: Detailed Task Breakdown
## AI Tool Foundation - Day-by-Day Guide

**Goal**: Get ChatGPT connected to accounting operations  
**Deliverable**: Upload invoice → AI extracts data → suggests journal entry

---

## DAY 1: Core Tool Infrastructure

### Morning: Setup & Tool Skeleton

**Task 1.1**: Create accounting tools file
```bash
mkdir -p codex-rs/core/src/tools
touch codex-rs/core/src/tools/accounting.rs
```

**Add to file** `codex-rs/core/src/tools/mod.rs`:
```rust
#[cfg(feature = "ledger")]
pub mod accounting;
```

**Task 1.2**: Add ledger feature to `codex-rs/core/Cargo.toml`:
```toml
[features]
ledger = ["codex-accounting-api", "codex-ledger", "codex-policy", "codex-approvals"]

[dependencies]
codex-accounting-api = { path = "../codex-accounting-api", optional = true }
codex-ledger = { path = "../codex-ledger", optional = true }
```

### Afternoon: Implement First Tool

**Task 1.3**: Implement `CreateCompanyTool` in `accounting.rs`:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use codex_accounting_api::LedgerFacade;
use codex_ledger::{LedgerCreateCompanyParams, LedgerCurrency, LedgerFiscalCalendar, LedgerTenantContext};
use serde::{Deserialize, Serialize};
use super::{ToolHandler, ToolKind, ToolInvocation, ToolOutput, FunctionCallError};

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
        let args: CreateCompanyArgs = serde_json::from_str(&invocation.args)
            .map_err(|e| FunctionCallError::InvalidArgs(format!("Failed to parse args: {e}")))?;
        
        let currency = LedgerCurrency {
            code: args.base_currency.clone(),
            symbol: "$".into(), // TODO: lookup by currency code
            decimal_places: 2,
        };
        
        let params = LedgerCreateCompanyParams {
            name: args.name,
            base_currency: currency,
            fiscal_calendar: LedgerFiscalCalendar {
                periods_per_year: 12,
                opening_month: args.fiscal_year_opening_month,
            },
        };
        
        let tenant = LedgerTenantContext {
            user_id: "agent".into(),
            roles: vec!["admin".into()],
        };
        
        let response = self.facade
            .create_company(params, tenant)
            .await
            .map_err(|e| FunctionCallError::ExecutionError(format!("Failed to create company: {e}")))?;
        
        let output = serde_json::to_string(&response.company)
            .map_err(|e| FunctionCallError::SerializationError(format!("Failed to serialize: {e}")))?;
        
        Ok(ToolOutput {
            content: output,
            metadata: None,
        })
    }
}
```

**Test compilation**:
```bash
cd codex-rs
cargo build --features ledger -p codex-core
```

✓ **Day 1 Success**: Tool compiles, can be instantiated

---

## DAY 2: Complete Tool Set

### Morning: List & Account Tools

**Task 2.1**: Add `ListCompaniesTool` (similar pattern, ~60 lines)
**Task 2.2**: Add `UpsertAccountTool` (similar pattern, ~80 lines)
**Task 2.3**: Add `ListAccountsTool` (similar pattern, ~60 lines)

**Key pattern for all tools**:
1. Define Args struct with `#[derive(Deserialize)]`
2. Implement `ToolHandler` trait
3. Parse args → call facade → serialize response
4. Convert errors to `FunctionCallError`

### Afternoon: Entry Tools

**Task 2.4**: Add `PostJournalEntryTool` (~100 lines):

```rust
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
    memo: Option<String>,
}

pub struct PostJournalEntryTool {
    facade: Arc<LedgerFacade>,
}

#[async_trait]
impl ToolHandler for PostJournalEntryTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }
    
    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        let args: PostEntryArgs = serde_json::from_str(&invocation.args)?;
        
        // Validate balance
        let total_debits: i64 = args.lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = args.lines.iter().map(|l| l.credit_minor).sum();
        
        if total_debits != total_credits {
            return Err(FunctionCallError::InvalidArgs(
                format!("Entry not balanced: debits={total_debits}, credits={total_credits}")
            ));
        }
        
        // Convert to ledger types and post
        // ... (implementation)
        
        Ok(ToolOutput { content: output, metadata: None })
    }
}
```

**Task 2.5**: Add `ListEntriesTool` (~70 lines)
**Task 2.6**: Add `GetCompanyContextTool` - returns chart of accounts + recent entries (~80 lines)

✓ **Day 2 Success**: All 7 tools implemented

---

## DAY 3: Tool Registration & Testing

### Morning: Register Tools

**Task 3.1**: Update `codex-rs/core/src/tools/registry.rs`:

```rust
use std::sync::Arc;
use std::collections::HashMap;

#[cfg(feature = "ledger")]
use super::accounting::*;

pub struct ToolRegistry {
    handlers: HashMap<String, Arc<dyn ToolHandler>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Arc<dyn ToolHandler>> = HashMap::new();
        
        // Register existing tools
        // ...
        
        #[cfg(feature = "ledger")]
        {
            // Initialize facades
            let ledger_facade = Arc::new(LedgerFacade::new(/* ... */));
            
            // Register accounting tools
            handlers.insert(
                "create_company".into(),
                Arc::new(CreateCompanyTool::new(ledger_facade.clone())),
            );
            handlers.insert(
                "list_companies".into(),
                Arc::new(ListCompaniesTool::new(ledger_facade.clone())),
            );
            handlers.insert(
                "upsert_account".into(),
                Arc::new(UpsertAccountTool::new(ledger_facade.clone())),
            );
            handlers.insert(
                "list_accounts".into(),
                Arc::new(ListAccountsTool::new(ledger_facade.clone())),
            );
            handlers.insert(
                "post_journal_entry".into(),
                Arc::new(PostJournalEntryTool::new(ledger_facade.clone())),
            );
            handlers.insert(
                "list_entries".into(),
                Arc::new(ListEntriesTool::new(ledger_facade.clone())),
            );
            handlers.insert(
                "get_company_context".into(),
                Arc::new(GetCompanyContextTool::new(ledger_facade.clone())),
            );
        }
        
        Self { handlers }
    }
    
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        self.handlers.get(name).cloned()
    }
}
```

### Afternoon: Unit Tests

**Task 3.2**: Add test module to `accounting.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    
    fn mock_facade() -> Arc<LedgerFacade> {
        // Create mock or use in-memory implementation
        todo!("Create mock facade")
    }
    
    #[tokio::test]
    async fn create_company_tool_works() {
        let tool = CreateCompanyTool::new(mock_facade());
        
        let invocation = ToolInvocation {
            args: r#"{"name": "Test Corp", "base_currency": "USD", "fiscal_year_opening_month": 1}"#.into(),
            call_id: "test-1".into(),
        };
        
        let result = tool.handle(invocation).await;
        assert!(result.is_ok(), "Tool should succeed");
        
        let output = result.unwrap();
        let company: serde_json::Value = serde_json::from_str(&output.content).unwrap();
        assert_eq!(company["name"], "Test Corp");
    }
    
    #[tokio::test]
    async fn post_entry_validates_balance() {
        let tool = PostJournalEntryTool::new(mock_facade());
        
        let unbalanced = r#"{
            "company_id": "comp-1",
            "date": "2024-01-01",
            "memo": "Test",
            "lines": [
                {"account_code": "1000", "debit_minor": 100, "credit_minor": 0},
                {"account_code": "5000", "debit_minor": 0, "credit_minor": 50}
            ]
        }"#;
        
        let invocation = ToolInvocation {
            args: unbalanced.into(),
            call_id: "test-2".into(),
        };
        
        let result = tool.handle(invocation).await;
        assert!(result.is_err(), "Should reject unbalanced entry");
        assert!(result.unwrap_err().to_string().contains("not balanced"));
    }
    
    // Add 3-5 more tests for other tools
}
```

**Run tests**:
```bash
cargo test --features ledger -p codex-core accounting
```

✓ **Day 3 Success**: Tools registered, tests pass

---

## DAY 4: ChatGPT Function Definitions

### Morning: Define Function Schemas

**Task 4.1**: Create `codex-rs/core/src/tools/definitions.rs` or update existing:

```rust
use serde_json::json;

pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub fn accounting_function_definitions() -> Vec<FunctionDefinition> {
    vec![
        FunctionDefinition {
            name: "create_company".into(),
            description: "Create a new company in the accounting system. Sets up the company with a base currency and fiscal calendar. Returns the company ID.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The legal name of the company"
                    },
                    "base_currency": {
                        "type": "string",
                        "description": "ISO 4217 currency code (USD, EUR, GBP, etc.)"
                    },
                    "fiscal_year_opening_month": {
                        "type": "integer",
                        "description": "Month when fiscal year starts (1-12, where 1=January)",
                        "minimum": 1,
                        "maximum": 12
                    }
                },
                "required": ["name", "base_currency", "fiscal_year_opening_month"]
            }),
        },
        FunctionDefinition {
            name: "post_journal_entry".into(),
            description: "Post a journal entry to the ledger. Entry must balance (total debits = total credits). Use this to record financial transactions. Returns the entry ID.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "company_id": {
                        "type": "string",
                        "description": "The company ID where this entry should be posted"
                    },
                    "date": {
                        "type": "string",
                        "description": "Transaction date in YYYY-MM-DD format"
                    },
                    "memo": {
                        "type": "string",
                        "description": "Description of the transaction"
                    },
                    "lines": {
                        "type": "array",
                        "description": "Journal entry lines (must balance)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "account_code": {
                                    "type": "string",
                                    "description": "Account code from chart of accounts"
                                },
                                "debit_minor": {
                                    "type": "integer",
                                    "description": "Debit amount in minor currency units (cents)"
                                },
                                "credit_minor": {
                                    "type": "integer",
                                    "description": "Credit amount in minor currency units (cents)"
                                },
                                "memo": {
                                    "type": "string",
                                    "description": "Optional line-specific memo"
                                }
                            },
                            "required": ["account_code", "debit_minor", "credit_minor"]
                        }
                    }
                },
                "required": ["company_id", "date", "memo", "lines"]
            }),
        },
        FunctionDefinition {
            name: "get_company_context".into(),
            description: "Get the chart of accounts and recent transactions for a company. Use this before suggesting journal entries to understand available accounts and transaction patterns.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "company_id": {
                        "type": "string",
                        "description": "The company ID"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of recent transactions to include (default 50)",
                        "default": 50
                    }
                },
                "required": ["company_id"]
            }),
        },
        // Add definitions for other 4 tools...
    ]
}
```

### Afternoon: Test with ChatGPT

**Task 4.2**: Create manual test script `test_chatgpt_tools.rs`:

```rust
// Simple test to verify ChatGPT can call tools
#[tokio::test]
async fn chatgpt_can_create_company() {
    let registry = ToolRegistry::new();
    let chatgpt = ChatGPTClient::new(/* ... */);
    
    let functions = accounting_function_definitions();
    
    let response = chatgpt.chat_with_functions(
        "You are an accounting assistant.",
        "Create a company called 'Demo Corp' with USD currency and fiscal year starting in January.",
        &functions
    ).await.unwrap();
    
    // ChatGPT should return a function call
    assert!(response.function_call.is_some());
    assert_eq!(response.function_call.unwrap().name, "create_company");
}
```

✓ **Day 4 Success**: Function definitions work with ChatGPT

---

## DAY 5: Document Agent - Part 1

### All Day: Setup Agent Structure

**Task 5.1**: Create accounting module structure:
```bash
mkdir -p codex-rs/core/src/accounting
touch codex-rs/core/src/accounting/{mod.rs,document_agent.rs,types.rs}
```

**Task 5.2**: Define types in `types.rs`:

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceData {
    pub vendor: String,
    pub invoice_number: Option<String>,
    pub date: NaiveDate,
    pub line_items: Vec<LineItem>,
    pub subtotal: f64,
    pub tax_amount: f64,
    pub total: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    pub description: String,
    pub quantity: Option<f64>,
    pub unit_price: Option<f64>,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntrySuggestion {
    pub lines: Vec<SuggestedLine>,
    pub memo: String,
    pub confidence: f32,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedLine {
    pub account_code: String,
    pub account_name: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
}

impl JournalEntrySuggestion {
    pub fn is_balanced(&self) -> bool {
        let total_debits: i64 = self.lines.iter().map(|l| l.debit_minor).sum();
        let total_credits: i64 = self.lines.iter().map(|l| l.credit_minor).sum();
        total_debits == total_credits
    }
}
```

**Task 5.3**: Create agent skeleton in `document_agent.rs`:

```rust
use std::sync::Arc;
use codex_ocr::OcrService;
use codex_accounting_api::LedgerFacade;
use chatgpt::ChatGPTClient;
use super::types::*;

pub struct DocumentAgent {
    ocr_service: Arc<dyn OcrService>,
    ledger_facade: Arc<LedgerFacade>,
    chatgpt_client: Arc<ChatGPTClient>,
}

impl DocumentAgent {
    pub fn new(
        ocr_service: Arc<dyn OcrService>,
        ledger_facade: Arc<LedgerFacade>,
        chatgpt_client: Arc<ChatGPTClient>,
    ) -> Self {
        Self {
            ocr_service,
            ledger_facade,
            chatgpt_client,
        }
    }
    
    pub async fn process_document(
        &self,
        upload_id: &str,
        company_id: &str,
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // Step 1: Get OCR text
        let ocr_result = self.ocr_service.process(upload_id).await?;
        
        // Step 2: Extract structured invoice data
        let invoice_data = self.extract_invoice_data(&ocr_result.text, company_id).await?;
        
        // Step 3: Get company context (chart of accounts)
        let accounts = self.get_chart_of_accounts(company_id).await?;
        
        // Step 4: Suggest journal entry
        let suggestion = self.suggest_journal_entry(&invoice_data, &accounts).await?;
        
        // Step 5: Validate
        if !suggestion.is_balanced() {
            return Err("AI suggested unbalanced entry".into());
        }
        
        Ok(suggestion)
    }
    
    async fn extract_invoice_data(
        &self,
        ocr_text: &str,
        company_id: &str,
    ) -> Result<InvoiceData, Box<dyn std::error::Error>> {
        todo!("Implement extraction - Day 6")
    }
    
    async fn get_chart_of_accounts(
        &self,
        company_id: &str,
    ) -> Result<Vec<Account>, Box<dyn std::error::Error>> {
        // Call ledger facade to get accounts
        todo!("Implement - Day 6")
    }
    
    async fn suggest_journal_entry(
        &self,
        invoice_data: &InvoiceData,
        accounts: &[Account],
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        todo!("Implement suggestion - Day 6")
    }
}
```

✓ **Day 5 Success**: Agent structure in place, types defined

---

## WEEK 1 DELIVERABLE TEST

At end of week, this should work:

```rust
#[tokio::test]
async fn week_1_integration_test() {
    // Setup
    let agent = DocumentAgent::new(ocr_service, ledger_facade, chatgpt_client);
    
    // Sample invoice text
    let invoice_text = "INVOICE\nAcme Office Supplies\nInvoice: INV-001\nDate: 2024-01-15\nOffice supplies: $500.00\nTax: $40.00\nTotal: $540.00";
    
    // Upload and process
    let upload_id = upload_document(invoice_text).await?;
    let company_id = "test-company-1";
    
    // Process document
    let suggestion = agent.process_document(&upload_id, company_id).await?;
    
    // Verify
    assert!(suggestion.is_balanced(), "Entry should balance");
    assert!(suggestion.confidence > 0.7, "Confidence should be reasonable");
    assert!(suggestion.lines.len() >= 2, "Should have at least 2 lines");
    
    println!("✅ Week 1 Complete!");
    println!("AI suggested entry:");
    println!("{}", serde_json::to_string_pretty(&suggestion)?);
}
```

**Expected output**:
```json
{
  "lines": [
    {
      "account_code": "5100",
      "account_name": "Office Supplies Expense",
      "debit_minor": 50000,
      "credit_minor": 0
    },
    {
      "account_code": "2200",
      "account_name": "Sales Tax Payable",
      "debit_minor": 4000,
      "credit_minor": 0
    },
    {
      "account_code": "1000",
      "account_name": "Cash",
      "debit_minor": 0,
      "credit_minor": 54000
    }
  ],
  "memo": "Office supplies purchase from Acme Office Supplies",
  "confidence": 0.92,
  "reasoning": "Coded to Office Supplies Expense as these are operational purchases. Sales tax recorded as liability. Cash credited for payment."
}
```

---

## TROUBLESHOOTING

### Issue: Tools don't compile
- Check feature flag is enabled: `--features ledger`
- Verify all dependencies in Cargo.toml
- Check `ToolHandler` trait signature matches

### Issue: Tests fail
- Ensure mock facades return valid data
- Check test uses `pretty_assertions`
- Verify async runtime (tokio) is configured

### Issue: ChatGPT doesn't call functions
- Check function definitions have correct schema
- Verify system prompt mentions available functions
- Test with simpler prompts first

---

## NEXT: Week 2 Preview

**Day 6-7**: Complete document agent (extraction + suggestion prompts)  
**Day 8-9**: Posting agent + policy integration  
**Day 10**: Testing + CLI commands

By end of Week 2, you'll have: upload → extract → suggest → policy → post/approve
