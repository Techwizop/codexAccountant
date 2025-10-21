# 🚀 START HERE - Next Coding Session

## Quick Status
✅ **Completed**: 7 accounting tools created (479 lines)  
⏭️ **Next**: Wire up real types and register tools  
⏱️ **Time**: 4-6 hours for next 10 tasks

---

## 📋 COPY THIS TO YOUR AI AGENT

```
Continue Codex Accounting implementation from Day 1 Afternoon.

CONTEXT:
Project: Autonomous accounting software with ChatGPT
Status: 7 accounting tools created in codex-rs/core/src/tools/accounting.rs
Progress: Day 1 Morning complete (30%), need to finish Day 1-2

CURRENT FILE STATE:
✅ codex-rs/core/src/tools/accounting.rs - 479 lines, 7 tools with placeholder types
✅ codex-rs/core/src/tools/mod.rs - accounting module added
✅ codex-rs/core/Cargo.toml - ledger feature configured
❌ Tools use placeholder types: type LedgerFacade = (); type TenantContext = ();
❌ Tools not registered in ToolRegistry
❌ No ChatGPT function schemas yet

YOUR MISSION: Complete these 10 tasks in order

---

TASK 1: Fix Type Imports in accounting.rs
Location: codex-rs/core/src/tools/accounting.rs (lines 6-8)

Current problem:
```rust
type LedgerFacade = ();
type TenantContext = ();
```

Action:
1. Search codebase for real LedgerFacade (likely in codex-accounting-api crate)
2. Search for TenantContext (likely in codex-ledger crate)
3. Replace placeholders with: use codex_accounting_api::...; use codex_ledger::...;
4. Check if ToolInvocation/ToolOutput/ToolHandler already exist in core
5. If yes, import them instead of local definitions

Success: cargo check --features ledger -p codex-core passes

---

TASK 2: Register Tools in Registry
Location: codex-rs/core/src/tools/registry.rs

Action:
1. Read registry.rs to understand how tools are registered
2. Add at top: #[cfg(feature = "ledger")] use super::accounting::*;
3. In registry initialization, add:
```rust
#[cfg(feature = "ledger")]
{
    let ledger_facade = Arc::new(/* create or get facade */);
    registry.insert("create_company", Arc::new(CreateCompanyTool::new(ledger_facade.clone())));
    registry.insert("list_companies", Arc::new(ListCompaniesTool::new(ledger_facade.clone())));
    registry.insert("upsert_account", Arc::new(UpsertAccountTool::new(ledger_facade.clone())));
    registry.insert("list_accounts", Arc::new(ListAccountsTool::new(ledger_facade.clone())));
    registry.insert("post_journal_entry", Arc::new(PostJournalEntryTool::new(ledger_facade.clone())));
    registry.insert("list_entries", Arc::new(ListEntriesTool::new(ledger_facade.clone())));
    registry.insert("get_company_context", Arc::new(GetCompanyContextTool::new(ledger_facade.clone())));
}
```

Success: Tools accessible via registry lookup

---

TASK 3: Create Function Definitions for ChatGPT
Location: Find or create codex-rs/core/src/tools/definitions.rs

Action: Add function that returns OpenAI-compatible schemas:
```rust
pub fn accounting_function_definitions() -> Vec<FunctionDefinition> {
    vec![
        FunctionDefinition {
            name: "create_company".into(),
            description: "Create a new company with fiscal calendar".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Company name"},
                    "base_currency": {"type": "string", "description": "ISO 4217 code (USD, EUR, etc)"},
                    "fiscal_year_opening_month": {"type": "integer", "minimum": 1, "maximum": 12}
                },
                "required": ["name", "base_currency", "fiscal_year_opening_month"]
            }),
        },
        // Add 6 more function definitions for other tools
    ]
}
```

Template: Match JSON properties to Args structs from accounting.rs
Success: All 7 tools have function definitions

---

TASK 4: Test Compilation
Commands:
```bash
cd codex-rs
cargo check --features ledger -p codex-core
cargo test --features ledger -p codex-core accounting
```

Expected: Compiles without errors, 5 tests pass
If fails: Fix errors before continuing

---

TASK 5: Create Accounting Module Structure
New files needed:
- codex-rs/core/src/accounting/mod.rs
- codex-rs/core/src/accounting/types.rs
- codex-rs/core/src/accounting/document_agent.rs

Step 5a: Create directory
```bash
mkdir -p codex-rs/core/src/accounting
```

Step 5b: Create mod.rs
```rust
#![cfg(feature = "ledger")]
pub mod types;
pub mod document_agent;
pub use types::*;
pub use document_agent::*;
```

Step 5c: Wire into core/src/lib.rs
Add: #[cfg(feature = "ledger")] pub mod accounting;

Success: Module structure compiles

---

TASK 6: Define Data Types
Location: codex-rs/core/src/accounting/types.rs

Copy exactly:
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
        let debits: i64 = self.lines.iter().map(|l| l.debit_minor).sum();
        let credits: i64 = self.lines.iter().map(|l| l.credit_minor).sum();
        debits == credits
    }
}
```

Success: Types compile

---

TASK 7: Create Document Agent Skeleton
Location: codex-rs/core/src/accounting/document_agent.rs

```rust
use std::sync::Arc;
use super::types::*;

pub struct DocumentAgent {
    // Add fields when dependencies ready
}

impl DocumentAgent {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn process_document(
        &self,
        upload_id: &str,
        company_id: &str,
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // 1. OCR text (mock for now)
        let ocr_text = "INVOICE\nMock Vendor\nTotal: $100";
        
        // 2. Extract data
        let invoice = self.extract_invoice_data(ocr_text, company_id).await?;
        
        // 3. Get accounts (mock for now)
        let accounts = vec![];
        
        // 4. Suggest entry
        let suggestion = self.suggest_journal_entry(&invoice, &accounts).await?;
        
        // 5. Validate
        if !suggestion.is_balanced() {
            return Err("Unbalanced entry".into());
        }
        
        Ok(suggestion)
    }
    
    async fn extract_invoice_data(
        &self,
        ocr_text: &str,
        _company_id: &str,
    ) -> Result<InvoiceData, Box<dyn std::error::Error>> {
        // TODO: ChatGPT prompt
        // Mock return for now
        Ok(InvoiceData {
            vendor: "Mock Vendor".into(),
            invoice_number: Some("INV-001".into()),
            date: chrono::Utc::now().naive_utc().date(),
            line_items: vec![],
            subtotal: 100.0,
            tax_amount: 8.0,
            total: 108.0,
            confidence: 0.9,
        })
    }
    
    async fn suggest_journal_entry(
        &self,
        invoice: &InvoiceData,
        _accounts: &[/* Account type */],
    ) -> Result<JournalEntrySuggestion, Box<dyn std::error::Error>> {
        // TODO: ChatGPT prompt
        // Mock return for now
        let total_minor = (invoice.total * 100.0) as i64;
        Ok(JournalEntrySuggestion {
            lines: vec![
                SuggestedLine {
                    account_code: "5000".into(),
                    account_name: "Expenses".into(),
                    debit_minor: total_minor,
                    credit_minor: 0,
                },
                SuggestedLine {
                    account_code: "1000".into(),
                    account_name: "Cash".into(),
                    debit_minor: 0,
                    credit_minor: total_minor,
                },
            ],
            memo: format!("Payment to {}", invoice.vendor),
            confidence: 0.85,
            reasoning: "Standard expense entry".into(),
        })
    }
}
```

Success: Agent compiles, returns mock data

---

TASK 8: Add ChatGPT Extraction Prompt
Location: In extract_invoice_data method

Replace mock with:
```rust
let system_prompt = "You are an expert accountant. Extract invoice data accurately.";

let user_prompt = format!(
    "Extract from this invoice:\n\
    - Vendor name\n\
    - Invoice number\n\
    - Date (YYYY-MM-DD)\n\
    - Line items\n\
    - Subtotal, tax, total\n\
    - Confidence (0.0-1.0)\n\n\
    OCR Text:\n{}\n\n\
    Return JSON: {{\"vendor\": \"...\", \"date\": \"YYYY-MM-DD\", ...}}",
    ocr_text
);

// Call ChatGPT here when client available
// For now, keep mock or add TODO
```

Success: Prompt well-formatted

---

TASK 9: Add ChatGPT Suggestion Prompt
Location: In suggest_journal_entry method

```rust
let system_prompt = "You are an expert accountant. Suggest balanced journal entries.";

let user_prompt = format!(
    "Invoice: {} from {}, total ${:.2}\n\
    Available accounts:\n{}\n\n\
    Suggest balanced entry (debits = credits) in minor units (cents).\n\
    Return JSON: {{\"lines\": [...], \"memo\": \"...\", \"confidence\": 0.0-1.0}}",
    invoice_data.invoice_number.as_deref().unwrap_or("N/A"),
    invoice_data.vendor,
    invoice_data.total,
    // format accounts list here
);

// Call ChatGPT when available
```

Success: Prompt clear and structured

---

TASK 10: Write Tests
Location: At end of document_agent.rs

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn entry_validates_balance() {
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
            reasoning: "Test".into(),
        };
        assert!(suggestion.is_balanced());
    }
    
    #[tokio::test]
    async fn agent_processes_document() {
        let agent = DocumentAgent::new();
        let result = agent.process_document("upload-1", "comp-1").await;
        assert!(result.is_ok());
        let suggestion = result.unwrap();
        assert!(suggestion.is_balanced());
    }
}
```

Success: Tests pass

---

FINAL VERIFICATION

Run:
```bash
cd codex-rs
cargo test --features ledger -p codex-core
cargo clippy --features ledger -p codex-core
just fmt
```

Expected output:
✓ All tests pass (5 original + new tests)
✓ No clippy warnings
✓ Code formatted

---

COMPLETION CHECKLIST

Phase 1 - Day 1-2 Complete When:
- [x] Task 1: Type imports fixed
- [x] Task 2: Tools registered
- [x] Task 3: Function definitions created
- [x] Task 4: Compilation succeeds
- [x] Task 5: Module structure in place
- [x] Task 6: Data types defined
- [x] Task 7: Agent skeleton created
- [x] Task 8: Extraction prompt added
- [x] Task 9: Suggestion prompt added
- [x] Task 10: Tests written and passing

---

AFTER COMPLETION

Update IMPLEMENTATION_LOG.md with:
- Date and time
- Tasks completed (1-10)
- Any issues encountered
- Next session: Start Phase 1 - Day 3 (Posting Agent)

THEN READ: CONTINUE_IMPLEMENTATION.md for detailed Week 2 guidance
```

---

## 💡 TIPS FOR SUCCESS

1. **Read before writing**: Always read existing files first
2. **Match patterns**: Copy existing tool/test patterns
3. **Test incrementally**: Run `cargo check` after each task
4. **Mock when blocked**: Use TODO and mock data if dependencies missing
5. **Document issues**: Note any blockers in IMPLEMENTATION_LOG.md

---

## 📞 IF YOU GET STUCK

**Can't find types?**
→ Use ripgrep: `rg "struct LedgerFacade" codex-rs/`

**Don't know function signature?**
→ Read similar tools in registry.rs

**Tests fail?**
→ Check error message, fix validation logic

**ChatGPT client unknown?**
→ Mock it for now, add TODO comment

---

## 🎯 SUCCESS CRITERIA

After 4-6 hours you should have:
✅ All 7 tools using real types
✅ Tools registered and callable
✅ ChatGPT knows how to use tools
✅ Document agent can extract and suggest
✅ All tests passing
✅ Ready for Phase 2 (App Server API)

---

**⏱️ Estimated Time**: 4-6 hours  
**📊 Progress After**: 50% of Week 1 complete  
**🎁 Deliverable**: Working document extraction with AI

---

## 🚀 START NOW

Tell your AI agent:

**"I'm starting next session of Codex Accounting. Read NEXT_SESSION.md and execute all 10 tasks in the copy-paste section. Start with Task 1: Fix Type Imports in accounting.rs. Work incrementally and test after each task."**
