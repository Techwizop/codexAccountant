# Phase 2 Progress: App Server API Layer

**Phase**: App Server API Integration  
**Start Date**: October 21, 2025  
**Status**: In Progress - Task 1 Complete

---

## ✅ Completed Tasks

### Task 1: Protocol Definitions ✅ (100% Complete)

**File Modified**: `codex-rs/app-server-protocol/src/protocol.rs`

#### Added to ClientRequest enum:
- ✅ `LedgerListCompanies` - List all companies with optional search
- ✅ `LedgerListAccounts` - List accounts for a company with filters
- ✅ `LedgerListEntries` - List journal entries with pagination
- ✅ `LedgerGetCompanyContext` - Get comprehensive company context for AI
- ✅ `LedgerProcessDocument` - Process uploaded document with AI

#### New Protocol Types Added:

**List Operations:**
```rust
- LedgerListCompaniesParams / LedgerListCompaniesResponse
- LedgerListAccountsParams / LedgerListAccountsResponse  
- LedgerListEntriesParams / LedgerListEntriesResponse
```

**Context & Processing:**
```rust
- LedgerGetCompanyContextParams / LedgerGetCompanyContextResponse
- LedgerPolicyRules (auto-post settings, confidence thresholds)
- LedgerProcessDocumentParams / LedgerProcessDocumentResponse
- LedgerJournalEntrySuggestion (AI suggestion structure)
- LedgerSuggestedLine (suggested journal entry lines)
```

**Features:**
- ✅ All types have `#[derive(TS)]` for TypeScript generation
- ✅ Proper serde serialization with camelCase
- ✅ Optional fields with `skip_serializing_if`
- ✅ Default values for pagination (limit=50)
- ✅ HashMap support for vendor mappings

---

## 📋 Remaining Tasks

### Task 2: App Server Message Handlers (NEXT)
**Status**: Not Started  
**Estimated Time**: 3-4 hours

**What to do:**
1. Create `codex-rs/app-server/src/accounting_handlers.rs`
2. Implement handler functions for each new method:
   - `list_companies()`
   - `list_accounts()`
   - `list_entries()`
   - `get_company_context()`
   - `process_document()` - wire to DocumentAgent

**Example Pattern:**
```rust
pub async fn list_companies(
    &self,
    params: LedgerListCompaniesParams,
    tenant: TenantContext,
) -> Result<LedgerListCompaniesResponse, String> {
    // Call ledger service
    let companies = self.ledger_service
        .list_companies(tenant, params.search)
        .await
        .map_err(|e| format!("Failed to list companies: {e}"))?;
        
    Ok(LedgerListCompaniesResponse { companies })
}
```

### Task 3: Wire into Message Processor
**Status**: Not Started  
**Estimated Time**: 2 hours

**What to do:**
1. Modify `codex-rs/app-server/src/message_processor.rs`
2. Add method routing for new operations
3. Add feature flag guards: `#[cfg(feature = "ledger")]`

### Task 4: Integration Tests
**Status**: Not Started  
**Estimated Time**: 2-3 hours

**What to do:**
1. Create `codex-rs/app-server/tests/accounting_api_test.rs`
2. Test each endpoint end-to-end
3. Mock ChatGPT responses for document processing

### Task 5: CLI Command Integration
**Status**: Not Started  
**Estimated Time**: 2 hours

**What to do:**
1. Create `codex-rs/cli/src/accounting_cmd.rs`
2. Add subcommands:
   - `codex accounting list-companies`
   - `codex accounting process-invoice <file>`
   - `codex accounting ledger <company>`

### Task 6: TypeScript Bindings
**Status**: Protocol types ready for export  
**Estimated Time**: 1 hour

**What to do:**
1. Run `cargo test` in `app-server-protocol`
2. Verify TypeScript files generated in `bindings/`
3. Copy to web UI project

---

## 📁 Files Modified So Far

### Phase 1 (Previously Completed)
- `codex-rs/core/src/tools/accounting.rs` - 7 accounting tools
- `codex-rs/core/src/tools/spec.rs` - Tool registration
- `codex-rs/core/src/accounting/` - Document agent module
- `codex-rs/core/src/lib.rs` - Module wiring

### Phase 2 (Current)
1. ✅ `codex-rs/app-server-protocol/src/protocol.rs` - Added protocol types

### Phase 2 (Pending)
2. ⏳ `codex-rs/app-server/src/accounting_handlers.rs` (NEW)
3. ⏳ `codex-rs/app-server/src/message_processor.rs` (MODIFY)
4. ⏳ `codex-rs/app-server/tests/accounting_api_test.rs` (NEW)
5. ⏳ `codex-rs/cli/src/accounting_cmd.rs` (NEW)
6. ⏳ `codex-rs/cli/src/main.rs` (MODIFY)

---

## 🎯 What Works Now

### Available API Endpoints (Protocol Level)
All protocol types are defined and ready for:

1. **ledgerListCompanies** - List all companies
   ```json
   {
     "method": "ledgerListCompanies",
     "params": { "search": "Acme" }
   }
   ```

2. **ledgerListAccounts** - Get chart of accounts
   ```json
   {
     "method": "ledgerListAccounts",
     "params": { "company_id": "comp-123" }
   }
   ```

3. **ledgerListEntries** - Get journal entries
   ```json
   {
     "method": "ledgerListEntries",
     "params": {
       "company_id": "comp-123",
       "start_date": "2024-01-01",
       "limit": 50
     }
   }
   ```

4. **ledgerGetCompanyContext** - Get AI context
   ```json
   {
     "method": "ledgerGetCompanyContext",
     "params": { "company_id": "comp-123" }
   }
   ```

5. **ledgerProcessDocument** - Process uploaded document
   ```json
   {
     "method": "ledgerProcessDocument",
     "params": {
       "upload_id": "upload-456",
       "company_id": "comp-123"
     }
   }
   ```

### What's Missing
- ❌ Handler implementations (Task 2)
- ❌ Message routing (Task 3)
- ❌ Tests (Task 4)
- ❌ CLI commands (Task 5)

---

## 🚀 Next Steps (Immediate)

**Priority 1: Implement Handlers**
```bash
# Create the handler file
touch codex-rs/app-server/src/accounting_handlers.rs

# Add to app-server/src/lib.rs:
#[cfg(feature = "ledger")]
pub mod accounting_handlers;
```

**What the handler needs:**
- Access to `LedgerFacade` for ledger operations
- Access to `DocumentAgent` for document processing
- Error handling and conversion
- Tenant context management

**Pattern to follow:**
1. Parse params from JSON-RPC
2. Create tenant context
3. Call underlying service (LedgerFacade or DocumentAgent)
4. Convert results to protocol types
5. Return response

---

## 📊 Phase 2 Progress Summary

**Overall Progress**: 20% (Task 1 of 6 complete)

| Task | Status | Time Spent | Est. Remaining |
|------|--------|------------|----------------|
| 1. Protocol Definitions | ✅ Complete | 30 min | 0 |
| 2. Handler Implementation | ⏳ Next | 0 | 3-4 hrs |
| 3. Message Routing | ⏳ Pending | 0 | 2 hrs |
| 4. Integration Tests | ⏳ Pending | 0 | 2-3 hrs |
| 5. CLI Commands | ⏳ Pending | 0 | 2 hrs |
| 6. TypeScript Bindings | ⏳ Pending | 0 | 1 hr |
| **Total** | **20%** | **0.5 hrs** | **10-13 hrs** |

---

## ✨ Key Achievements

1. **Complete Protocol Coverage**: All accounting operations now have well-defined protocol types
2. **TypeScript Ready**: All types annotated with `#[derive(TS)]` for automatic binding generation
3. **Follows Existing Patterns**: Integrated seamlessly with existing Codex protocol structure
4. **Extensible**: Easy to add more operations in the future
5. **Well-Documented**: Proper naming and structure for clarity

---

## 🎬 How to Continue

**Option 1: Continue with AI Agent**
```
I'm continuing Phase 2 of Codex Accounting. 

COMPLETED:
- Task 1: Protocol definitions (app-server-protocol/src/protocol.rs)

NEXT:
- Task 2: Implement accounting_handlers.rs with methods for:
  - list_companies()
  - list_accounts()
  - list_entries()
  - get_company_context()
  - process_document()

Start by creating codex-rs/app-server/src/accounting_handlers.rs
Follow the pattern from existing app-server handlers.
Wire up LedgerFacade and DocumentAgent from Phase 1.
```

**Option 2: Manual Implementation**
1. Read PHASE_2_IMPLEMENTATION.md for detailed examples
2. Follow Task 2 instructions
3. Reference existing handlers in app-server/src/

---

**Status**: Phase 2 - 20% Complete  
**Next**: Task 2 - Handler Implementation  
**Blockers**: None
