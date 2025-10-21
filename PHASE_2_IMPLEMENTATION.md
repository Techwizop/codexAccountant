# Phase 2: App Server API Layer Implementation

**Phase**: App Server API Integration  
**Duration**: 2 weeks estimated  
**Prerequisites**: ✅ Phase 1 Complete (AI Tools & Document Agent)

---

## 🎯 Phase 2 Objectives

Expose accounting operations via JSON-RPC API for:
- Web UI consumption
- CLI command integration  
- External API access

---

## 📋 Implementation Tasks

### Task 1: Protocol Definitions (2-3 hours)
**File**: `codex-rs/app-server-protocol/src/lib.rs`

Add protocol types for all accounting operations:

#### 1.1 Company Management
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerCreateCompanyParams {
    pub name: String,
    pub base_currency: String,
    pub fiscal_year_opening_month: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerCreateCompanyResponse {
    pub company: Company,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerListCompaniesParams {
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerListCompaniesResponse {
    pub companies: Vec<Company>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: String,
    pub name: String,
    pub base_currency: String,
    pub fiscal_year_opening_month: u8,
    pub created_at: String,
}
```

#### 1.2 Chart of Accounts
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerUpsertAccountParams {
    pub company_id: String,
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub parent_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerUpsertAccountResponse {
    pub account: Account,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerListAccountsParams {
    pub company_id: String,
    pub account_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerListAccountsResponse {
    pub accounts: Vec<Account>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub company_id: String,
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub parent_code: Option<String>,
    pub balance_minor: i64,
}
```

#### 1.3 Journal Entries
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerPostEntryParams {
    pub company_id: String,
    pub date: String,
    pub memo: String,
    pub lines: Vec<EntryLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryLine {
    pub account_code: String,
    pub debit_minor: i64,
    pub credit_minor: i64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerPostEntryResponse {
    pub entry: JournalEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerListEntriesParams {
    pub company_id: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub account_code: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerListEntriesResponse {
    pub entries: Vec<JournalEntry>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: String,
    pub company_id: String,
    pub date: String,
    pub memo: String,
    pub lines: Vec<EntryLine>,
    pub posted_at: String,
    pub posted_by: String,
}
```

#### 1.4 Document Processing
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDocumentParams {
    pub upload_id: String,
    pub company_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDocumentResponse {
    pub suggestion: JournalEntrySuggestion,
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
```

---

### Task 2: App Server Message Handlers (3-4 hours)
**File**: `codex-rs/app-server/src/accounting_handlers.rs` (NEW)

Create handler functions for each method:

```rust
use codex_app_server_protocol::*;
use codex_accounting_api::LedgerFacade;
use codex_ledger::{InMemoryLedgerService, TenantContext};
use std::sync::Arc;

pub struct AccountingHandlers {
    ledger_facade: Arc<LedgerFacade>,
}

impl AccountingHandlers {
    pub fn new(ledger_facade: Arc<LedgerFacade>) -> Self {
        Self { ledger_facade }
    }
    
    pub async fn create_company(
        &self,
        params: LedgerCreateCompanyParams,
        tenant: TenantContext,
    ) -> Result<LedgerCreateCompanyResponse, String> {
        // Call facade
        let company = self.ledger_facade
            .create_company(params, tenant)
            .await
            .map_err(|e| format!("Failed to create company: {e}"))?;
            
        Ok(LedgerCreateCompanyResponse { company })
    }
    
    pub async fn list_companies(
        &self,
        params: LedgerListCompaniesParams,
        tenant: TenantContext,
    ) -> Result<LedgerListCompaniesResponse, String> {
        // Implementation
        todo!()
    }
    
    // ... more handlers
}
```

---

### Task 3: Wire into Message Processor (2 hours)
**File**: `codex-rs/app-server/src/message_processor.rs`

Add accounting methods to the message router:

```rust
// Add to imports
#[cfg(feature = "ledger")]
use crate::accounting_handlers::AccountingHandlers;

// In message processor setup
#[cfg(feature = "ledger")]
let accounting_handlers = AccountingHandlers::new(ledger_facade);

// Add to match statement
match method {
    "create_company" => {
        #[cfg(feature = "ledger")]
        {
            let params: LedgerCreateCompanyParams = parse_params(params)?;
            let response = accounting_handlers.create_company(params, tenant).await?;
            Ok(serde_json::to_value(response)?)
        }
        #[cfg(not(feature = "ledger"))]
        Err("Accounting features not enabled".into())
    },
    "list_companies" => { /* ... */ },
    "upsert_account" => { /* ... */ },
    "list_accounts" => { /* ... */ },
    "post_journal_entry" => { /* ... */ },
    "list_entries" => { /* ... */ },
    "process_document" => { /* ... */ },
    // ... more methods
}
```

---

### Task 4: Integration Tests (2-3 hours)
**File**: `codex-rs/app-server/tests/accounting_api_test.rs` (NEW)

Create end-to-end tests:

```rust
use codex_app_server_protocol::*;
use codex_core::protocol::*;
use serde_json::json;

#[tokio::test]
async fn test_create_company_flow() {
    // Setup test server
    let server = setup_test_server().await;
    
    // Create company request
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "create_company",
        "params": {
            "name": "Test Corp",
            "base_currency": "USD",
            "fiscal_year_opening_month": 1
        }
    });
    
    let response = server.call(request).await;
    
    assert!(response["result"]["company"]["id"].is_string());
    assert_eq!(response["result"]["company"]["name"], "Test Corp");
}

#[tokio::test]
async fn test_document_processing_flow() {
    let server = setup_test_server().await;
    
    // Upload document
    // Process document
    // Verify suggestion
    
    todo!()
}
```

---

### Task 5: CLI Command Integration (2 hours)
**File**: `codex-rs/cli/src/accounting_cmd.rs` (NEW)

Add CLI commands:

```rust
use clap::{Args, Subcommand};

#[derive(Debug, Args)]
pub struct AccountingArgs {
    #[command(subcommand)]
    command: AccountingCommand,
}

#[derive(Debug, Subcommand)]
enum AccountingCommand {
    /// Create a new company
    CreateCompany {
        name: String,
        #[arg(long)]
        currency: String,
        #[arg(long)]
        fiscal_month: u8,
    },
    
    /// List companies
    ListCompanies {
        #[arg(long)]
        search: Option<String>,
    },
    
    /// Upload and process invoice
    ProcessInvoice {
        file: String,
        #[arg(long)]
        company_id: String,
    },
    
    /// Show ledger entries
    Ledger {
        #[arg(long)]
        company_id: String,
        #[arg(long)]
        start_date: Option<String>,
        #[arg(long)]
        end_date: Option<String>,
    },
}

pub async fn handle_accounting_command(args: AccountingArgs) -> Result<(), Error> {
    match args.command {
        AccountingCommand::CreateCompany { name, currency, fiscal_month } => {
            // Call app server API
            todo!()
        },
        // ... more commands
    }
}
```

---

### Task 6: TypeScript Bindings (1 hour)
**File**: `codex-rs/app-server-protocol/src/lib.rs`

Add ts-rs annotations:

```rust
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LedgerCreateCompanyParams {
    pub name: String,
    pub base_currency: String,
    pub fiscal_year_opening_month: u8,
}
```

Generate bindings:
```bash
cd codex-rs/app-server-protocol
cargo test
# Types exported to bindings/ folder
```

---

## 🔧 Implementation Order

**Priority 1 (Day 1-2):**
1. ✅ Protocol definitions for company & accounts
2. ✅ Basic message handlers
3. ✅ Wire into message processor
4. ✅ Test with curl/Postman

**Priority 2 (Day 3-4):**
5. ✅ Journal entry endpoints
6. ✅ Document processing endpoint
7. ✅ Integration tests
8. ✅ Error handling

**Priority 3 (Day 5):**
9. ✅ CLI commands
10. ✅ TypeScript bindings
11. ✅ Documentation

---

## 📁 Files to Create/Modify

### New Files
1. `codex-rs/app-server/src/accounting_handlers.rs`
2. `codex-rs/app-server/tests/accounting_api_test.rs`
3. `codex-rs/cli/src/accounting_cmd.rs`

### Modified Files
1. `codex-rs/app-server-protocol/src/lib.rs` - Add protocol types
2. `codex-rs/app-server/src/lib.rs` - Export accounting_handlers
3. `codex-rs/app-server/src/message_processor.rs` - Add method handlers
4. `codex-rs/cli/src/main.rs` - Add accounting subcommand

---

## ✅ Success Criteria

- [ ] All protocol types defined with serde serialization
- [ ] All 7 core accounting endpoints working
- [ ] Document processing endpoint integrated with DocumentAgent
- [ ] Integration tests passing
- [ ] CLI commands functional
- [ ] TypeScript bindings generated
- [ ] Error handling comprehensive
- [ ] Documentation complete

---

## 🚀 Quick Start Commands

```bash
# Test protocol types
cd codex-rs/app-server-protocol
cargo test

# Test app server handlers
cd codex-rs/app-server
cargo test --features ledger accounting

# Run app server with accounting
cargo run --features ledger -- --port 8080

# Test via curl
curl -X POST http://localhost:8080/api \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "create_company",
    "params": {
      "name": "Test Corp",
      "base_currency": "USD",
      "fiscal_year_opening_month": 1
    }
  }'

# Generate TypeScript bindings
cd codex-rs/app-server-protocol
cargo test
ls bindings/*.ts
```

---

## 📝 Next Steps After Phase 2

Once Phase 2 is complete, you'll be ready for:

**Phase 3: Web UI Development**
- React app consuming JSON-RPC API
- Real-time updates via WebSocket
- Document upload and processing UI
- Approval workflow interface

---

**Status**: Ready to implement  
**Estimated Time**: 1-2 weeks  
**Blockers**: None (Phase 1 complete)
