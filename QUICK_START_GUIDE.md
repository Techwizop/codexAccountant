# Quick Start Guide - Immediate Action Items

## Summary

Your Codex accounting codebase is **80% complete** on the backend. You have:

✅ **12 production-quality Rust crates** for accounting operations
✅ **Complete double-entry ledger** with multi-currency support
✅ **Bank reconciliation engine** with smart matching
✅ **Approval workflows** with multi-stage chains
✅ **Policy engine** for autonomous decision-making
✅ **OCR & document classification** infrastructure
✅ **ChatGPT integration** in Codex core

## What's Missing

❌ **AI Agent Integration** - ChatGPT not connected to accounting modules
❌ **Web UI** - Planning doc exists but no React app
❌ **App Server APIs** - Accounting methods not exposed via JSON-RPC

---

## Week 1-2: Critical Path

### Task 1: Create AI Accounting Tools (3-4 days)

**File**: `codex-rs/core/src/tools/accounting.rs`

```rust
use codex_accounting_api::LedgerFacade;
use codex_ledger::TenantContext;

pub struct CreateCompanyTool {
    facade: Arc<LedgerFacade>,
}

impl ToolHandler for CreateCompanyTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }
    
    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        // Parse args, call facade, return result
    }
}

// Also implement:
// - PostJournalEntryTool
// - ListAccountsTool
// - UploadDocumentTool
// - ProposeJournalEntryTool (uses ChatGPT for suggestions)
```

**Register in**: `codex-rs/core/src/tools/registry.rs`

### Task 2: Document Extraction Agent (3-4 days)

**File**: `codex-rs/core/src/accounting/document_agent.rs`

```rust
pub struct DocumentAgent {
    ocr_service: Arc<OcrService>,
    ledger_facade: Arc<LedgerFacade>,
    chatgpt_client: Arc<ChatGPTClient>,
}

impl DocumentAgent {
    pub async fn process_document(&self, upload_id: &str, company_id: &str) -> Result<JournalEntry> {
        // 1. Retrieve uploaded document
        // 2. Run OCR extraction
        // 3. ChatGPT: extract structured data (vendor, amount, date, line items)
        // 4. ChatGPT: suggest journal entry with reasoning
        // 5. Return proposed entry with confidence
    }
}
```

**ChatGPT System Prompt Addition**:
```
You are an expert accountant helping automate bookkeeping. When given a document:

1. Extract: vendor, invoice number, date, total amount, tax amount, line items
2. Suggest: which accounts to debit/credit using the chart of accounts provided
3. Explain: your reasoning and confidence level (0-1.0)

Always follow double-entry accounting rules. Format output as JSON.
```

### Task 3: App Server Protocol (2-3 days)

**File**: `codex-rs/app-server-protocol/src/lib.rs`

Add these structures:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export)]
pub struct CreateCompanyParams {
    pub name: String,
    pub base_currency: LedgerCurrency,
    pub fiscal_calendar: LedgerFiscalCalendar,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export)]
pub struct CreateCompanyResponse {
    pub company: LedgerCompany,
}

// Add 15-20 more method types for accounting operations
```

### Task 4: App Server Handlers (2-3 days)

**File**: `codex-rs/app-server/src/accounting_handlers.rs`

```rust
pub async fn handle_accounting_method(
    method: &str,
    params: Value,
    ledger_facade: &LedgerFacade,
) -> Result<Value, JSONRPCError> {
    match method {
        "accounting/createCompany" => {
            let params: CreateCompanyParams = serde_json::from_value(params)?;
            let response = ledger_facade.create_company(params, tenant).await?;
            Ok(serde_json::to_value(response)?)
        }
        "accounting/postEntry" => { /* ... */ }
        "accounting/listApprovals" => { /* ... */ }
        // etc.
    }
}
```

**Update**: `codex-rs/app-server/src/message_processor.rs`
```rust
if method.starts_with("accounting/") {
    #[cfg(feature = "ledger")]
    return handle_accounting_method(method, params, &self.ledger_facade).await;
}
```

---

## Week 3-4: UI Foundation

### Task 5: Initialize React App (1 day)

```bash
cd apps/codex-gui
pnpm create vite . --template react-ts
pnpm install

# Install key dependencies
pnpm add @tanstack/react-query @tanstack/react-router zustand
pnpm add tailwindcss @tailwindcss/forms lucide-react
pnpm add recharts date-fns clsx

# Initialize Tailwind
pnpx tailwindcss init -p
```

### Task 6: JSON-RPC Client (2 days)

**File**: `apps/codex-gui/src/lib/rpc-client.ts`

```typescript
import { AppServerProtocol } from '@codex/protocol'; // Generated types

export class RPCClient {
  private ws: WebSocket;
  private requestId = 0;
  private pending = new Map<number, {resolve, reject}>();
  
  async call<T>(method: string, params: any): Promise<T> {
    const id = ++this.requestId;
    return new Promise((resolve, reject) => {
      this.pending.set(id, {resolve, reject});
      this.ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
    });
  }
  
  // Methods matching protocol
  async createCompany(params: CreateCompanyParams): Promise<CreateCompanyResponse> {
    return this.call('accounting/createCompany', params);
  }
}
```

### Task 7: Core UI Pages (5-6 days)

1. **Dashboard** (`src/pages/Dashboard.tsx`)
   - Company selector
   - Recent activity feed
   - Quick stats (pending approvals, unreconciled items)

2. **Upload Center** (`src/pages/UploadCenter.tsx`)
   - Drag-drop component
   - Upload progress with stages
   - Document list with status

3. **Approval Queue** (`src/pages/Approvals.tsx`)
   - List with filters
   - Document preview panel
   - Approve/reject actions

4. **Transactions** (`src/pages/Transactions.tsx`)
   - Journal entries table
   - Filters and search
   - Detail drawer

---

## Testing Your MVP

### End-to-End Test Scenario

1. **Start Services**
   ```bash
   # Terminal 1: App Server
   cd codex-rs
   cargo run --bin codex-app-server --features ledger
   
   # Terminal 2: Web UI
   cd apps/codex-gui
   pnpm dev
   ```

2. **Test Flow**
   - Open http://localhost:5173
   - Create company "Test Corp"
   - Upload sample invoice PDF
   - Watch: OCR → ChatGPT extraction → Policy evaluation
   - Review in approval queue
   - Approve → verify in transaction ledger

3. **Validate AI**
   - Check extracted vendor/amount accuracy
   - Review suggested account mappings
   - Verify confidence scores
   - Test with 10 different invoice types

---

## Sample ChatGPT Prompt for Document Extraction

```json
{
  "role": "system",
  "content": "You are an expert accountant. Extract structured data from documents."
}
{
  "role": "user", 
  "content": "Document text:\n\nINVOICE\nAcme Office Supplies\nInvoice #: INV-2024-001\nDate: March 15, 2024\n\nOffice chairs (5x) ... $500.00\nDesk supplies ... $75.00\n\nSubtotal: $575.00\nSales Tax (8%): $46.00\nTotal: $621.00\n\n---\n\nExtract: vendor, invoice_number, date, line_items, subtotal, tax, total.\nSuggest journal entry using these accounts:\n- 1000 Cash\n- 5100 Office Supplies Expense\n- 2200 Sales Tax Payable\n\nReturn JSON with confidence scores."
}
```

**Expected Response**:
```json
{
  "extraction": {
    "vendor": "Acme Office Supplies",
    "invoice_number": "INV-2024-001",
    "date": "2024-03-15",
    "line_items": [
      {"description": "Office chairs", "quantity": 5, "amount": 500.00},
      {"description": "Desk supplies", "amount": 75.00}
    ],
    "subtotal": 575.00,
    "tax": 46.00,
    "total": 621.00,
    "confidence": 0.95
  },
  "suggested_entry": {
    "lines": [
      {"account": "5100", "debit": 575.00, "credit": 0},
      {"account": "2200", "debit": 46.00, "credit": 0},
      {"account": "1000", "debit": 0, "credit": 621.00}
    ],
    "memo": "Office supplies purchase from Acme",
    "confidence": 0.90,
    "reasoning": "Coded to Office Supplies Expense (5100) as these are operational purchases. Sales tax goes to liability account (2200) until remitted. Cash credit (1000) for payment."
  }
}
```

---

## Key Files to Review

### Backend (Rust)
1. `codex-rs/codex-ledger/src/lib.rs` - Core ledger types
2. `codex-rs/codex-accounting-api/src/facade.rs` - API layer
3. `codex-rs/codex-accounting-api/src/demo.rs` - Example usage
4. `codex-rs/core/src/tools/registry.rs` - Tool registration
5. `codex-rs/app-server/src/message_processor.rs` - JSON-RPC routing

### Specs
1. `codex-rs/specs/autonomous-accounting-spec.md` - Product requirements
2. `codex-rs/specs/tasks.md` - Implementation phases

### UI
1. `apps/codex-gui/README.md` - UI planning doc

---

## Quick Wins for Demo

To get a working demo ASAP (1-2 weeks):

1. **Skip UI temporarily** - Use CLI commands:
   ```bash
   codex accounting create-company "Demo Corp"
   codex accounting upload-invoice ~/invoice.pdf --company demo-corp
   codex accounting approvals list
   codex accounting approvals approve <id>
   codex accounting ledger show
   ```

2. **Use in-memory storage** - Already implemented, no DB setup needed

3. **Mock OCR initially** - Use `MockOcrProvider` with pre-defined text

4. **Test with ChatGPT directly** - Validate prompts work before full integration

---

## Success Criteria

### Week 2 Checkpoint
- ✅ Can call accounting tools from Codex CLI
- ✅ ChatGPT successfully extracts invoice data
- ✅ Proposed journal entries balance (debits = credits)
- ✅ Policy engine routes to auto-post or approval

### Week 4 Checkpoint  
- ✅ Web UI connects to app server
- ✅ Can upload document through UI
- ✅ Approval queue displays pending items
- ✅ Can approve and see transaction posted

### Week 6-8 Checkpoint (MVP)
- ✅ Full flow: upload → OCR → AI → policy → approval → ledger
- ✅ 80%+ accuracy on test invoices
- ✅ Bank reconciliation works
- ✅ Basic reports (trial balance, P&L)

---

## Need Help?

### Debugging
- **ChatGPT responses**: Check `OTEL_EXPORTER_OTLP_ENDPOINT` logs
- **Ledger errors**: Review `codex-ledger/src/lib.rs` validation logic
- **UI connection**: Inspect WebSocket frames in browser DevTools

### Resources
- Existing accounting tests: `codex-rs/codex-ledger/src/lib.rs` (line 477+)
- Demo data generator: `codex-rs/codex-accounting-api/src/demo.rs`
- Protocol examples: `codex-rs/app-server-protocol/src/lib.rs`

### Architecture Decisions
- **Why Rust backend?** - Memory safety, concurrency, performance
- **Why React UI?** - Modern, maintainable, large ecosystem
- **Why JSON-RPC?** - Simple, type-safe with TypeScript bindings
- **Why ChatGPT?** - Best-in-class reasoning for complex accounting logic

---

## Next Steps Summary

1. **Week 1**: Implement AI accounting tools (Task 1-2)
2. **Week 2**: Wire up app server APIs (Task 3-4)
3. **Week 3**: Initialize React app and RPC client (Task 5-6)
4. **Week 4**: Build core UI pages (Task 7)
5. **Week 5+**: Iterate, test, refine

**You have an excellent foundation. The path forward is clear and achievable.** 🚀
