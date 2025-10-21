# Continue Implementation - Guided Prompt for AI Coding Agents

**Project**: Codex Accounting - Autonomous Agent  
**Current Phase**: Phase 1 - AI Tool Foundation  
**Progress**: Day 1 Morning Complete (30%)  
**Next**: Day 1 Afternoon → Week 2

---

## 🎯 IMMEDIATE CONTEXT

### What's Been Completed ✅

1. **All 7 accounting tools created** (`codex-rs/core/src/tools/accounting.rs`)
   - CreateCompanyTool, ListCompaniesTool, UpsertAccountTool, ListAccountsTool
   - PostJournalEntryTool, ListEntriesTool, GetCompanyContextTool
   - 479 lines with validation and 5 unit tests

2. **Module structure configured**
   - Added to `tools/mod.rs` with feature flag
   - Cargo.toml updated with `ledger` feature
   - Optional dependencies added

3. **Ready for compilation**
   - Code follows all Rust conventions
   - Tests written
   - Validation logic complete

### What Needs Doing ⏭️

**CRITICAL PATH** (blocks everything else):
- Wire up real `LedgerFacade` (currently placeholder)
- Register tools in `ToolRegistry`
- Define ChatGPT function schemas
- Test compilation

---

## 📋 SUPER GUIDED PROMPT - COPY THIS TO AI AGENT

```
I'm continuing implementation of Codex Accounting. Current state:

COMPLETED:
- File: codex-rs/core/src/tools/accounting.rs (479 lines, 7 tools, 5 tests)
- File: codex-rs/core/src/tools/mod.rs (added accounting module)
- File: codex-rs/core/Cargo.toml (added ledger feature)

CURRENT ISSUES:
- Tool types are placeholder: type LedgerFacade = ()
- Tools not registered in ToolRegistry
- No ChatGPT function definitions yet
- Not tested with real facades

YOUR TASKS (in order):

TASK 1: Fix Tool Type Imports
File: codex-rs/core/src/tools/accounting.rs

Problem: Currently using placeholder types at top of file:
```rust
type LedgerFacade = ();
type TenantContext = ();
```

Action:
1. Find where `LedgerFacade` is actually defined (likely `codex-accounting-api`)
2. Find where `TenantContext` is defined (likely `codex-ledger`)
3. Replace placeholder types with proper imports
4. Check if `ToolInvocation`, `ToolOutput`, `ToolHandler`, `ToolKind` exist elsewhere in codebase
5. If they do, import them instead of defining locally

Expected imports:
```rust
use codex_accounting_api::LedgerFacade;
use codex_ledger::TenantContext;
// Maybe also: use super::{ToolHandler, ToolInvocation, ToolOutput, ToolKind};
```

Success criteria: File compiles with real types

---

TASK 2: Register Tools in ToolRegistry
File: codex-rs/core/src/tools/registry.rs

Current state: Registry exists but doesn't include accounting tools

Action:
1. Read codex-rs/core/src/tools/registry.rs to understand structure
2. Add accounting tool imports under feature flag:
   ```rust
   #[cfg(feature = "ledger")]
   use super::accounting::*;
   ```
3. In registry initialization (likely `new()` or `default()`):
   - Create/get LedgerFacade instance
   - Register all 7 tools with names:
     * "create_company" -> CreateCompanyTool
     * "list_companies" -> ListCompaniesTool
     * "upsert_account" -> UpsertAccountTool
     * "list_accounts" -> ListAccountsTool
     * "post_journal_entry" -> PostJournalEntryTool
     * "list_entries" -> ListEntriesTool
     * "get_company_context" -> GetCompanyContextTool

Example code pattern:
```rust
#[cfg(feature = "ledger")]
{
    let ledger_facade = Arc::new(/* get or create facade */);
    registry.insert("create_company", Arc::new(CreateCompanyTool::new(ledger_facade.clone())));
    registry.insert("list_companies", Arc::new(ListCompaniesTool::new(ledger_facade.clone())));
    // ... etc for all 7 tools
}
```

Success criteria: Tools callable via registry.get("create_company")

---

TASK 3: Create ChatGPT Function Definitions
File: codex-rs/core/src/tools/definitions.rs (or similar)

Action:
1. Find where function definitions are stored for ChatGPT
2. Create function for accounting tools:
   ```rust
   pub fn accounting_function_definitions() -> Vec<FunctionDefinition>
   ```
3. Define JSON schema for each tool

Template for ONE tool (repeat for all 7):
```rust
FunctionDefinition {
    name: "create_company".into(),
    description: "Create a new company in the accounting system with chart of accounts and fiscal calendar. Returns the company ID and details.".into(),
    parameters: json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "The legal name of the company"
            },
            "base_currency": {
                "type": "string",
                "description": "ISO 4217 currency code (e.g., USD, EUR, GBP)"
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
}
```

All 7 tools need definitions. Match properties to Args structs in accounting.rs.

Success criteria: ChatGPT knows how to call all accounting tools

---

TASK 4: Test Compilation
Commands to run:
```bash
cd codex-rs
cargo check --features ledger -p codex-core
cargo test --features ledger -p codex-core accounting
just fmt
just fix -p codex-core
```

Expected output:
- ✓ Compilation succeeds
- ✓ All 5 tests pass
- ✓ No clippy warnings

If errors: Fix them before proceeding

---

TASK 5: Create Document Agent Module
File: codex-rs/core/src/accounting/mod.rs (NEW)
File: codex-rs/core/src/accounting/types.rs (NEW)
File: codex-rs/core/src/accounting/document_agent.rs (NEW)

Action 5.1: Create directory and module file
```bash
mkdir codex-rs/core/src/accounting
```

File: codex-rs/core/src/accounting/mod.rs
```rust
#![cfg(feature = "ledger")]

pub mod types;
pub mod document_agent;

pub use types::*;
pub use document_agent::*;
```

Action 5.2: Define types
File: codex-rs/core/src/accounting/types.rs

Copy this exactly:
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

Action 5.3: Create document agent skeleton
File: codex-rs/core/src/accounting/document_agent.rs

Copy this structure:
```rust
use std::sync::Arc;
use super::types::*;

// These will need proper imports:
// use codex_ocr::OcrService;
// use codex_accounting_api::LedgerFacade;
// use some_chatgpt_client::ChatGPTClient;

pub struct DocumentAgent {
    // ocr_service: Arc<dyn OcrService>,
    // ledger_facade: Arc<LedgerFacade>,
    // chatgpt_client: Arc<ChatGPTClient>,
}

impl DocumentAgent {
    pub fn new(
        // ocr_service: Arc<dyn OcrService>,
        // ledger_facade: Arc<LedgerFacade>,
        // chatgpt_client: Arc<ChatGPTClient>,
    ) -> Self {
        Self {
            // ocr_service,
            // ledger_facade,
            // chatgpt_client,
        }
    }
    
    pub async fn process_document(
        &self,
        upload_id: &str,
        company_id: &str,
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // TODO: Implement full flow
        // 1. Get OCR text
        // 2. Extract invoice data
        // 3. Get chart of accounts
        // 4. Suggest journal entry
        // 5. Validate balance
        
        unimplemented!("Document processing coming in next task")
    }
    
    async fn extract_invoice_data(
        &self,
        ocr_text: &str,
        _company_id: &str,
    ) -> Result<InvoiceData, Box<dyn std::error::Error>> {
        // ChatGPT extraction prompt goes here
        unimplemented!("Extraction coming next")
    }
    
    async fn suggest_journal_entry(
        &self,
        invoice_data: &InvoiceData,
        accounts: &[Account], // Need to import Account type
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // ChatGPT suggestion prompt goes here
        unimplemented!("Suggestion coming next")
    }
}
```

Action 5.4: Wire into core
File: codex-rs/core/src/lib.rs

Add near top:
```rust
#[cfg(feature = "ledger")]
pub mod accounting;
```

Success criteria: 
- Module compiles
- Types defined
- Agent structure in place

---

TASK 6: Implement ChatGPT Extraction Prompt
File: codex-rs/core/src/accounting/document_agent.rs

Fill in `extract_invoice_data` method:

```rust
async fn extract_invoice_data(
    &self,
    ocr_text: &str,
    _company_id: &str,
) -> Result<InvoiceData, Box<dyn std::error::Error>> {
    let system_prompt = "You are an expert accountant specializing in invoice processing. \
        Extract structured data from invoice text with high accuracy.";
    
    let user_prompt = format!(
        "Extract the following information from this invoice OCR text:\n\
        - Vendor name (the company providing goods/services)\n\
        - Invoice number (if present)\n\
        - Date (convert to YYYY-MM-DD format)\n\
        - Line items with descriptions and amounts\n\
        - Subtotal (before tax)\n\
        - Tax amount\n\
        - Total amount\n\
        - Your confidence level (0.0 to 1.0)\n\n\
        OCR Text:\n{}\n\n\
        Return ONLY valid JSON matching this exact schema:\n\
        {{\n\
          \"vendor\": \"string\",\n\
          \"invoice_number\": \"string or null\",\n\
          \"date\": \"YYYY-MM-DD\",\n\
          \"line_items\": [\n\
            {{\n\
              \"description\": \"string\",\n\
              \"quantity\": number or null,\n\
              \"unit_price\": number or null,\n\
              \"amount\": number\n\
            }}\n\
          ],\n\
          \"subtotal\": number,\n\
          \"tax_amount\": number,\n\
          \"total\": number,\n\
          \"confidence\": number (0.0-1.0)\n\
        }}",
        ocr_text
    );
    
    // Call ChatGPT (you'll need to find actual client method):
    // let response = self.chatgpt_client.chat(system_prompt, &user_prompt).await?;
    
    // For now, return mock until ChatGPT client wired up:
    let response = r#"{
        "vendor": "Mock Vendor",
        "invoice_number": "INV-001",
        "date": "2024-01-15",
        "line_items": [{"description": "Mock item", "amount": 100.0}],
        "subtotal": 100.0,
        "tax_amount": 8.0,
        "total": 108.0,
        "confidence": 0.95
    }"#;
    
    let invoice_data: InvoiceData = serde_json::from_str(response)?;
    Ok(invoice_data)
}
```

Success criteria: Method compiles, returns valid InvoiceData

---

TASK 7: Implement ChatGPT Suggestion Prompt
File: codex-rs/core/src/accounting/document_agent.rs

Fill in `suggest_journal_entry` method:

```rust
async fn suggest_journal_entry(
    &self,
    invoice_data: &InvoiceData,
    accounts: &[Account],
) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
    // Format accounts list
    let accounts_list = accounts
        .iter()
        .map(|a| format!("{} - {} ({})", a.code, a.name, a.account_type))
        .collect::<Vec<_>>()
        .join("\n");
    
    let system_prompt = "You are an expert accountant. Suggest journal entries following \
        double-entry bookkeeping rules. Debits must always equal credits. \
        Amounts should be in minor currency units (cents).";
    
    let user_prompt = format!(
        "Invoice details:\n\
        Vendor: {}\n\
        Date: {}\n\
        Total: ${:.2}\n\
        Tax: ${:.2}\n\
        Subtotal: ${:.2}\n\n\
        Available accounts:\n{}\n\n\
        Task: Suggest a balanced journal entry to record this expense invoice.\n\
        The entry must follow double-entry bookkeeping (debits = credits).\n\
        Use minor currency units (multiply dollars by 100 for cents).\n\n\
        Return ONLY valid JSON:\n\
        {{\n\
          \"lines\": [\n\
            {{\n\
              \"account_code\": \"string\",\n\
              \"account_name\": \"string\",\n\
              \"debit_minor\": integer (cents),\n\
              \"credit_minor\": integer (cents)\n\
            }}\n\
          ],\n\
          \"memo\": \"string describing the transaction\",\n\
          \"confidence\": number (0.0-1.0),\n\
          \"reasoning\": \"string explaining your account choices\"\n\
        }}",
        invoice_data.vendor,
        invoice_data.date,
        invoice_data.total,
        invoice_data.tax_amount,
        invoice_data.subtotal,
        accounts_list
    );
    
    // Call ChatGPT:
    // let response = self.chatgpt_client.chat(system_prompt, &user_prompt).await?;
    
    // Mock for now:
    let total_minor = (invoice_data.total * 100.0) as i64;
    let response = format!(r#"{{
        "lines": [
            {{"account_code": "5000", "account_name": "Expenses", "debit_minor": {}, "credit_minor": 0}},
            {{"account_code": "1000", "account_name": "Cash", "debit_minor": 0, "credit_minor": {}}}
        ],
        "memo": "Expense from {}",
        "confidence": 0.9,
        "reasoning": "Standard expense entry"
    }}"#, total_minor, total_minor, invoice_data.vendor);
    
    let suggestion: JournalEntrySuggestion = serde_json::from_str(&response)?;
    
    // Validate balance
    if !suggestion.is_balanced() {
        return Err("AI suggested unbalanced entry".into());
    }
    
    Ok(suggestion)
}
```

Success criteria: Returns balanced suggestions

---

TASK 8: Implement Full Document Processing Flow
File: codex-rs/core/src/accounting/document_agent.rs

Fill in `process_document` method:

```rust
pub async fn process_document(
    &self,
    upload_id: &str,
    company_id: &str,
) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
    // Step 1: Get OCR text
    // For now, mock until OCR service integrated:
    let mock_ocr_text = "INVOICE\nAcme Supplies\nInvoice: INV-001\nDate: Jan 15, 2024\nTotal: $108.00";
    
    // Step 2: Extract structured data
    let invoice_data = self.extract_invoice_data(mock_ocr_text, company_id).await?;
    
    // Step 3: Get chart of accounts
    // Mock for now:
    let mock_accounts = vec![
        // You'll need to create proper Account structs
    ];
    
    // Step 4: Suggest journal entry
    let suggestion = self.suggest_journal_entry(&invoice_data, &mock_accounts).await?;
    
    // Step 5: Final validation
    if !suggestion.is_balanced() {
        return Err("Entry validation failed: not balanced".into());
    }
    
    if suggestion.confidence < 0.5 {
        return Err(format!("Confidence too low: {}", suggestion.confidence).into());
    }
    
    Ok(suggestion)
}
```

Success criteria: Full flow executes, returns suggestion

---

TASK 9: Write Tests for Document Agent
File: codex-rs/core/src/accounting/document_agent.rs

Add test module at end:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn suggestion_validates_balance() {
        let suggestion = JournalEntrySuggestion {
            lines: vec![
                SuggestedLine {
                    account_code: "5000".into(),
                    account_name: "Expense".into(),
                    debit_minor: 100,
                    credit_minor: 0,
                },
                SuggestedLine {
                    account_code: "1000".into(),
                    account_name: "Cash".into(),
                    debit_minor: 0,
                    credit_minor: 100,
                },
            ],
            memo: "Test".into(),
            confidence: 0.9,
            reasoning: "Test reasoning".into(),
        };
        
        assert!(suggestion.is_balanced());
    }
    
    #[tokio::test]
    async fn suggestion_detects_imbalance() {
        let suggestion = JournalEntrySuggestion {
            lines: vec![
                SuggestedLine {
                    account_code: "5000".into(),
                    account_name: "Expense".into(),
                    debit_minor: 100,
                    credit_minor: 0,
                },
                SuggestedLine {
                    account_code: "1000".into(),
                    account_name: "Cash".into(),
                    debit_minor: 0,
                    credit_minor: 50,
                },
            ],
            memo: "Test".into(),
            confidence: 0.9,
            reasoning: "Test".into(),
        };
        
        assert!(!suggestion.is_balanced());
    }
    
    // Add more tests for extract_invoice_data and suggest_journal_entry
}
```

Success criteria: Tests pass

---

TASK 10: Final Verification
Commands to run:
```bash
cd codex-rs
cargo test --features ledger -p codex-core
cargo clippy --features ledger -p codex-core
just fmt
```

Verify:
- ✓ All tests pass (original 5 + new tests)
- ✓ No clippy warnings
- ✓ Code formatted
- ✓ Document agent compiles
- ✓ Tools registered
- ✓ Function definitions created

Success criteria: Ready for Phase 2 (App Server API)

---

IMPORTANT NOTES:

1. **Find before writing**: Always read existing files first to understand patterns
2. **Match existing style**: Follow conventions in similar files
3. **Test incrementally**: Run `cargo check` after each task
4. **Mock when blocked**: If a dependency isn't ready, use mock data and add TODO
5. **Ask for help**: If types don't exist or you're stuck, document the blocker

REFERENCE FILES:
- WEEK_1_TASKS.md - Detailed day-by-day guide
- DEVELOPMENT_PLAN.md - All 250 tasks
- IMPLEMENTATION_LOG.md - What's been done
- codex-rs/core/src/tools/accounting.rs - Current tool implementation

AFTER COMPLETING ALL TASKS ABOVE:
Update IMPLEMENTATION_LOG.md with:
- Tasks completed
- Any issues encountered
- Next session starting point
```

---

## 🎯 CONDENSED PROMPT (for limited context)

```
Continue Codex Accounting implementation:

DONE: 7 accounting tools in codex-rs/core/src/tools/accounting.rs (479 lines)

TODO (in order):
1. Replace placeholder types (LedgerFacade, TenantContext) with real imports
2. Register 7 tools in tools/registry.rs under #[cfg(feature = "ledger")]
3. Create ChatGPT function definitions for all 7 tools (JSON schemas)
4. Test: cargo check --features ledger -p codex-core
5. Create accounting module: core/src/accounting/{mod.rs, types.rs, document_agent.rs}
6. Define InvoiceData, JournalEntrySuggestion types
7. Implement extract_invoice_data with ChatGPT prompt
8. Implement suggest_journal_entry with ChatGPT prompt
9. Implement process_document full flow
10. Write tests for document agent

Files to reference:
- WEEK_1_TASKS.md (detailed examples)
- DEVELOPMENT_PLAN.md (task list)
- accounting.rs (current implementation)

Success: All compiles, tests pass, document agent works
```

---

## 📚 KEY REFERENCE SECTIONS

### From WEEK_1_TASKS.md
- Day 1 template code (lines 30-180)
- Day 2 entry tools (lines 181-280)
- Day 4 function definitions (lines 400-550)
- Day 5 agent structure (lines 551-700)

### From DEVELOPMENT_PLAN.md
- Task 1-10: Foundation (lines 20-150)
- Task 11-20: Document agent (lines 151-300)
- Acceptance criteria for each

### From accounting.rs
- Tool pattern: struct + impl ToolHandler
- Validation examples (lines 150-250)
- Test pattern (lines 420-479)

---

## ✅ CHECKLIST FOR AI AGENT

Before starting:
- [ ] Read IMPLEMENTATION_LOG.md for current state
- [ ] Read WEEK_1_TASKS.md for examples
- [ ] Understand current placeholder types

During implementation:
- [ ] Task 1: Fix imports ✓
- [ ] Task 2: Register tools ✓
- [ ] Task 3: Function definitions ✓
- [ ] Task 4: Test compilation ✓
- [ ] Task 5: Agent structure ✓
- [ ] Task 6: Extraction prompt ✓
- [ ] Task 7: Suggestion prompt ✓
- [ ] Task 8: Process flow ✓
- [ ] Task 9: Tests ✓
- [ ] Task 10: Verify ✓

After completing:
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Update IMPLEMENTATION_LOG.md
- [ ] Ready for Phase 2

---

## 🚀 START COMMAND

**To begin next session, tell AI agent**:

"I'm continuing Codex Accounting implementation. Read CONTINUE_IMPLEMENTATION.md and execute tasks 1-10 in order. Current state: 7 accounting tools created, needs type imports and registration. Start with Task 1: Fix Tool Type Imports in accounting.rs"

---

**Status**: Ready for handoff to next coding session
**Estimated Time**: 4-6 hours for Tasks 1-10
**Deliverable**: Complete document agent with ChatGPT integration
