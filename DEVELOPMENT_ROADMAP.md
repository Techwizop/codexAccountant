# Codex Accounting Development Roadmap

## Executive Summary

This document outlines the development steps required to transform Codex CLI into a fully agentic accounting software application with ChatGPT as its accounting brain and a modern web UI.

## Current State Analysis

### ✅ Complete Backend Infrastructure (Rust)

The foundation is solid with 12 accounting-specific crates already implemented:

1. **`codex-ledger`** - Core double-entry accounting engine
   - Multi-company support with tenant isolation
   - Chart of accounts, journals, entries
   - Multi-currency with exchange rates
   - Period locking/closing workflows
   - Reconciliation status tracking
   - Audit trail generation

2. **`codex-accounting-api`** - Facade layer & integration
   - LedgerFacade for accounting operations
   - ReconciliationFacade for bank matching
   - ControlsFacade for approval workflows
   - TenancyFacade for multi-tenant operations
   - Telemetry and metrics collection
   - Demo data generation

3. **`codex-doc-ingest`** - Document upload pipeline
   - Signed URL generation for S3/storage
   - Ingestion job queuing
   - HTTP API routing with Axum
   - CLI harness for testing

4. **`codex-ocr`** - OCR & document classification
   - Document type detection (Invoice, Receipt, BankStatement, Payroll)
   - Confidence scoring system
   - Mock providers for development
   - Keyword-based classifier

5. **`codex-bank-ingest`** - Bank transaction parsing
   - CSV parser with configurable profiles
   - OFX (Open Financial Exchange) parser
   - Duplicate detection & deduplication
   - ISO-4217 currency validation
   - Transaction normalization

6. **`codex-reconcile`** - Reconciliation engine
   - Weighted scoring algorithm for matching
   - Session-based workflow management
   - Accept/reject/write-off actions
   - Partial acceptance for split transactions
   - Audit hook system
   - In-memory and pluggable storage

7. **`codex-approvals`** - Approval workflow system
   - Multi-stage approval chains
   - Role-based assignee filtering
   - SLA tracking and overdue monitoring
   - Priority queuing (Low/Normal/High)
   - Decision recording with rationale
   - Queue export for reporting

8. **`codex-policy`** - Policy evaluation engine
   - Auto-post threshold configuration
   - AI confidence floor requirements
   - Vendor/account allowlist/blocklist
   - Amount-based routing (auto/approval/reject)
   - Policy event telemetry
   - Durable storage with caching

9. **`codex-tenancy`** - Multi-tenant support
10. **`codex-audit-log`** - Immutable audit logging
11. **`codex-doc-store`** - Document storage abstraction
12. **`codex-backend-openapi-models`** - API type definitions

### ✅ Existing Codex CLI Infrastructure

- **ChatGPT Integration**: First-party ChatGPT API client (`chatgpt` crate)
- **Tool Registry**: Extensible tool handler system in `core/src/tools/`
- **Command Execution**: Shell, bash, unified exec with safety controls
- **MCP Support**: Model Context Protocol client/server
- **Protocol Layer**: JSON-RPC protocol with TypeScript bindings
- **App Server**: HTTP/WebSocket server for UI communication
- **TUI**: Terminal user interface with Ratatui
- **Authentication**: ChatGPT login flow

### 🚧 Partially Complete

1. **Specs & Documentation**
   - ✅ `specs/autonomous-accounting-spec.md` - Comprehensive product spec
   - ✅ `specs/tasks.md` - Phased roadmap (Phases 0-6)
   - ⚠️ Implementation lagging behind spec

2. **UI Shell**
   - ✅ `apps/codex-gui/README.md` - Planning document exists
   - ❌ No actual React/Vite application yet

3. **App Server Integration**
   - ⚠️ Accounting API crates exist but not exposed via app-server
   - ❌ No JSON-RPC methods for accounting operations

---

## 🎯 Remaining Development Steps

### Phase 1: AI Agent Integration (2-3 weeks)

**Goal**: Connect ChatGPT to accounting modules for autonomous document processing

#### 1.1 AI Tool Handlers for Accounting
- [ ] Create `codex-rs/core/src/tools/accounting.rs`
  - `create_company` tool
  - `upsert_account` tool
  - `post_journal_entry` tool
  - `list_audit_trail` tool
- [ ] Register accounting tools in `ToolRegistry`
- [ ] Add accounting context to system prompts

#### 1.2 Document Processing Pipeline
- [ ] Create `codex-rs/core/src/accounting_agent.rs`
  - Document upload → OCR → Classification flow
  - Extract structured data from OCR results
  - Generate chart of accounts suggestions
- [ ] Implement `extract_invoice_data` ChatGPT function
  - Vendor name, amount, date, line items, tax codes
  - Return structured JSON with confidence scores
- [ ] Implement `suggest_journal_entry` ChatGPT function
  - Input: extracted document data + chart of accounts
  - Output: proposed debits/credits with reasoning
  - Confidence scoring for policy evaluation

#### 1.3 Autonomous Posting Agent
- [ ] Create `codex-rs/core/src/accounting_agent/posting.rs`
  - Agent loop: monitor ingestion queue
  - Process document → extract → classify → propose → evaluate policy
  - Auto-post or enqueue for approval based on policy
- [ ] Integrate with `codex-policy` for decision routing
- [ ] Integrate with `codex-approvals` for human-in-loop
- [ ] Add telemetry and structured logging

#### 1.4 Context Management
- [ ] Implement company-specific memory
  - Previous vendor mappings (vendor X → account Y)
  - Learned patterns (recurring transactions)
  - User corrections and overrides
- [ ] Add to ChatGPT context window
  - Recent transactions for consistency
  - Chart of accounts for current company
  - Policy rules and thresholds

---

### Phase 2: App Server API Layer (2 weeks)

**Goal**: Expose accounting operations via JSON-RPC for UI consumption

#### 2.1 Protocol Definitions
- [ ] Add to `app-server-protocol/src/lib.rs`:
  ```rust
  // Company Management
  CreateCompanyParams, CreateCompanyResponse
  ListCompaniesParams, ListCompaniesResponse
  
  // Chart of Accounts
  UpsertAccountParams, UpsertAccountResponse
  ListAccountsParams, ListAccountsResponse
  
  // Journal Entries
  PostEntryParams, PostEntryResponse
  ListEntriesParams, ListEntriesResponse
  ReverseEntryParams, ReverseEntryResponse
  
  // Approvals
  ListApprovalsParams, ListApprovalsResponse
  ApproveEntryParams, ApproveEntryResponse
  
  // Reconciliation
  StartReconciliationParams, StartReconciliationResponse
  AcceptMatchParams, AcceptMatchResponse
  
  // Document Ingestion
  UploadDocumentParams, UploadDocumentResponse
  ListDocumentsParams, ListDocumentsResponse
  
  // Reporting
  GenerateTrialBalanceParams, TrialBalanceResponse
  GeneratePLParams, ProfitLossResponse
  GenerateBalanceSheetParams, BalanceSheetResponse
  ```

- [ ] Generate TypeScript bindings with `ts-rs`

#### 2.2 App Server Handlers
- [ ] Update `app-server/src/message_processor.rs`
  - Add accounting method handlers
  - Wire up to `LedgerFacade`, `ReconciliationFacade`, etc.
  - Add proper error handling and validation
- [ ] Add feature flag for accounting (`--features ledger`)
- [ ] Implement streaming updates for long operations
  - Document processing progress
  - Reconciliation status updates

#### 2.3 Testing & Mocking
- [ ] Create `app-server/tests/accounting_api_test.rs`
  - End-to-end JSON-RPC flow tests
  - Mock ChatGPT responses
  - Snapshot testing for responses

---

### Phase 3: Web UI Development (4-5 weeks)

**Goal**: Build modern React web application for accounting workflows

#### 3.1 Project Scaffold
- [ ] Initialize Vite + React 19 project in `apps/codex-gui/`
  ```bash
  cd apps/codex-gui
  pnpm create vite . --template react-ts
  ```
- [ ] Install dependencies:
  - TailwindCSS for styling
  - TanStack Query for server state
  - TanStack Router for routing
  - Zustand for UI state
  - Lucide React for icons
  - shadcn/ui for component library
  - Recharts for financial charts

#### 3.2 Core UI Infrastructure
- [ ] Set up JSON-RPC client
  - WebSocket connection to app-server
  - Request/response handling
  - Type-safe method invocation with protocol types
- [ ] Authentication flow
  - Reuse existing Codex login
  - Session management
  - Token refresh
- [ ] Layout & Navigation
  - Top nav with company switcher
  - Sidebar menu (Dashboard, Transactions, Reports, etc.)
  - Notification center
  - Dark mode support

#### 3.3 Core Features - Company Management
- [ ] Company selector dropdown
  - Search/filter
  - Recent companies
  - Create new company flow
- [ ] Company settings page
  - Chart of accounts editor
  - Policy configuration UI
  - Fiscal calendar settings

#### 3.4 Core Features - Document Ingestion
- [ ] Upload center page
  - Drag & drop file upload
  - Progress indicators with processing stages:
    1. Uploading
    2. OCR Processing
    3. Classification
    4. AI Extraction
    5. Policy Evaluation
    6. Queued / Auto-Posted
  - Document preview (PDF/image viewer)
  - Batch operations

#### 3.5 Core Features - Approval Queue
- [ ] Approval inbox page
  - Filterable list (pending, assigned to me, overdue)
  - Document preview side panel
  - AI reasoning display:
    - Extracted fields
    - Suggested account mappings
    - Confidence scores
  - Edit proposed entry inline
  - Approve / Reject / Request Clarification actions
  - Batch approval for similar items

#### 3.6 Core Features - Reconciliation Workspace
- [ ] Bank reconciliation page
  - Upload bank statement
  - Unmatched transactions list
  - Match candidates with scores
  - Side-by-side comparison
  - Accept / Reject / Write-off actions
  - Exception queue
  - Progress tracker

#### 3.7 Core Features - Transaction Ledger
- [ ] Journal entries list
  - Date range filter
  - Account filter
  - Search by description/amount
  - Drill-down to line details
  - Link to source document
- [ ] Entry detail view
  - Debit/credit lines
  - Audit trail (who posted, when, why)
  - Reverse entry action
  - Related documents

#### 3.8 Core Features - Reporting
- [ ] Trial Balance report
  - Period selector
  - Account drill-down
  - Export to CSV/PDF
- [ ] P&L Statement
  - Comparative periods
  - AI-generated commentary/insights
- [ ] Balance Sheet
  - As-of date selector
  - Asset/Liability/Equity sections
- [ ] Cash Flow Statement (future)
- [ ] Report builder (future)

#### 3.9 Core Features - Chat Assistant
- [ ] Floating chat dock
  - Always accessible
  - Context-aware (current page, selected items)
  - Quick commands:
    - "Post invoice INV-123"
    - "Show me Q1 revenue"
    - "Why was this coded to Marketing?"
  - Attach documents for clarification
- [ ] Chat history
- [ ] Suggested prompts based on context

---

### Phase 4: CLI/TUI Commands (1-2 weeks)

**Goal**: Add accounting commands to Codex CLI and TUI

#### 4.1 CLI Commands
- [ ] Add to `codex-rs/cli/src/commands/accounting.rs`:
  ```bash
  codex accounting company create "Acme Corp"
  codex accounting company list
  codex accounting upload invoice.pdf --company acme
  codex accounting ledger --company acme --month 2024-10
  codex accounting reconcile --company acme --bank statements.csv
  codex accounting approvals list
  codex accounting reports trial-balance --company acme
  ```

#### 4.2 TUI Screens
- [ ] Company selector screen
- [ ] Document upload progress screen
- [ ] Approval queue screen
  - Keyboard shortcuts
  - Document preview in terminal (if image)
- [ ] Transaction ledger browser
  - Fuzzy search
  - Vim-style navigation

---

### Phase 5: AI Enhancements (2-3 weeks)

**Goal**: Improve AI reasoning and context awareness

#### 5.1 Learning & Adaptation
- [ ] Implement feedback loop
  - When user corrects AI suggestion, store correction
  - Update context for future similar documents
  - Track accuracy metrics per vendor/document type
- [ ] Pattern recognition
  - Detect recurring transactions (rent, utilities)
  - Suggest automated rules
  - Confidence improvement over time

#### 5.2 AI-Powered Features
- [ ] Smart vendor matching
  - "ABC Co" = "ABC Company" = "ABC Corp"
  - Fuzzy matching with confidence
- [ ] Tax code suggestions
  - Based on account and jurisdiction
  - VAT/GST/Sales tax handling
- [ ] Accrual detection
  - Identify prepayments and deferrals
  - Suggest adjustment entries
- [ ] Period-end close assistant
  - Checklist generation
  - Outstanding items report
  - AI-guided close workflow

#### 5.3 Natural Language Reporting
- [ ] "Explain this variance" feature
  - Compare actuals vs. prior period
  - AI generates narrative explanation
- [ ] Management report generation
  - Executive summary with insights
  - Key metrics and trends
  - Risk flags and recommendations

---

### Phase 6: Integration & Persistence (2 weeks)

**Goal**: Add production-grade data persistence and integrations

#### 6.1 Database Integration
- [ ] Replace in-memory services with PostgreSQL
  - `codex-ledger` → SQL schema
  - `codex-approvals` → SQL schema
  - `codex-reconcile` → SQL schema
  - Migration tooling (diesel/sqlx)
- [ ] Add database connection pooling
- [ ] Transaction management and rollback

#### 6.2 Document Storage
- [ ] Implement S3-compatible storage for documents
  - Upload to S3/MinIO/Azure Blob
  - Versioning and retention policies
  - Encryption at rest
- [ ] Link documents to ledger entries
  - Attachment references
  - Preview generation (thumbnails)

#### 6.3 External Integrations (Roadmap)
- [ ] Bank feed APIs (Plaid, TrueLayer)
  - Automatic transaction import
  - Real-time balance updates
- [ ] Export integrations
  - QuickBooks export
  - Xero export
  - CSV/Excel formats

---

### Phase 7: Compliance & Security (1-2 weeks)

**Goal**: Ensure audit compliance and data security

#### 7.1 Audit & Compliance
- [ ] Immutable audit log implementation
  - Write-once storage
  - Cryptographic signatures
  - Tamper detection
- [ ] Audit report generation
  - Who did what, when, why
  - Change history for entries
  - Document access logs
- [ ] SOC2 controls documentation

#### 7.2 Security Hardening
- [ ] Role-based access control (RBAC)
  - Define roles: Admin, Accountant, Reviewer, Auditor
  - Permission system for operations
  - API-level enforcement
- [ ] Multi-factor authentication (MFA)
- [ ] Data encryption
  - At-rest encryption for sensitive fields
  - In-transit TLS enforcement
- [ ] Rate limiting and DDoS protection
- [ ] Security audit and penetration testing

---

### Phase 8: Testing & QA (Ongoing)

**Goal**: Comprehensive test coverage and validation

#### 8.1 Unit & Integration Tests
- [ ] Core accounting logic tests (already exists)
- [ ] AI agent workflow tests with mocked ChatGPT
- [ ] API endpoint tests
- [ ] Policy evaluation test matrix

#### 8.2 End-to-End Tests
- [ ] Playwright tests for UI workflows:
  - Document upload → approval → posting
  - Reconciliation complete flow
  - Report generation
- [ ] Golden dataset testing
  - Sample invoices, receipts, statements
  - Validate AI extraction accuracy
  - Regression testing

#### 8.3 Performance Testing
- [ ] Load testing for concurrent users
- [ ] Document processing throughput
- [ ] Large dataset handling (10K+ transactions)
- [ ] Report generation latency

---

## Development Priorities

### Immediate Next Steps (Week 1-2)

1. **AI Integration Foundation** [CRITICAL]
   - Create accounting tool handlers
   - Implement document extraction agent
   - Test with sample invoices

2. **App Server API** [HIGH]
   - Define protocol methods
   - Wire up existing accounting facades
   - Test JSON-RPC endpoints

3. **UI Scaffold** [HIGH]
   - Initialize Vite project
   - Set up routing and auth
   - Create layout shell

### Short-Term (Month 1)

4. **Core Document Flow**
   - Upload → OCR → AI Extraction → Approval → Post
   - End-to-end with real ChatGPT
   - Basic UI for monitoring

5. **Approval Workflow UI**
   - Queue view
   - Approval actions
   - Document preview

### Medium-Term (Month 2-3)

6. **Reconciliation Feature**
   - Bank import
   - Match suggestions
   - UI for acceptance/rejection

7. **Reporting**
   - Trial balance, P&L, balance sheet
   - Export capabilities
   - AI commentary

### Long-Term (Month 4+)

8. **Production Readiness**
   - Database persistence
   - Security hardening
   - External integrations
   - Beta testing with CPA firms

---

## Technical Architecture

### High-Level Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                         Web UI (React)                       │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │Dashboard │  │ Approvals│  │ Reconcile│  │ Reports  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │ JSON-RPC (WebSocket)
┌─────────────────────────────────────────────────────────────┐
│                    App Server (Rust)                         │
│  ┌────────────────────────────────────────────────────────┐ │
│  │           Message Processor & Routing                  │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                  Accounting API Layer                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Ledger  │  │Reconcile │  │ Approvals│  │  Policy  │   │
│  │  Facade  │  │  Facade  │  │  Facade  │  │  Engine  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                Core Accounting Services                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Ledger  │  │Bank Ingest│  │   OCR   │  │ Approvals│   │
│  │ Service  │  │  Service  │  │ Service  │  │ Service  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    ChatGPT Agent Layer                       │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Document Extraction → Classification → Posting Agent  │ │
│  │  - Tool Registry with Accounting Functions             │ │
│  │  - Context: Chart of Accounts, Prior Mappings, Policy │ │
│  │  - Reasoning: Confidence Scores, Explanations          │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────────┐
│                    Persistence Layer                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │PostgreSQL│  │S3 Storage│  │ Redis    │                  │
│  │(Ledger)  │  │(Documents)  │(Cache)   │                  │
│  └──────────┘  └──────────┘  └──────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Dependencies & Integrations

### Required External Services
- **ChatGPT API**: Document extraction, journal entry suggestions, report commentary
- **OCR Service**: Tesseract or cloud OCR (Azure AI, AWS Textract, Google Vision)
- **S3-Compatible Storage**: Document uploads (AWS S3, MinIO, Azure Blob)
- **PostgreSQL**: Persistent ledger data (future)
- **Redis**: Session cache and job queues (future)

### Optional Integrations (Roadmap)
- **Plaid/TrueLayer**: Bank feed automation
- **Stripe/PayPal**: Payment reconciliation
- **QuickBooks/Xero**: Import/export compatibility

---

## Success Metrics

### MVP (Minimum Viable Product) Goals
- [ ] Upload invoice → AI extracts → posts to ledger (80% accuracy)
- [ ] Bank statement → reconcile automatically (70% match rate)
- [ ] Approval workflow reduces manual review by 60%
- [ ] Web UI provides all core accounting functions
- [ ] ChatGPT explains any transaction on demand

### Production Readiness Goals
- [ ] 90%+ AI extraction accuracy on invoices
- [ ] <5 second document processing time
- [ ] Support 100+ concurrent users
- [ ] 99.9% uptime SLA
- [ ] SOC2 Type II compliance
- [ ] 5 CPA firms piloting successfully

---

## Risk Mitigation

### Technical Risks
1. **ChatGPT hallucinations in accounting data**
   - Mitigation: Confidence thresholds, human approval for low-confidence
   - Structured output validation against accounting rules

2. **OCR accuracy on poor quality documents**
   - Mitigation: Manual review queue for low confidence
   - User feedback loop to improve over time

3. **Multi-tenant data isolation bugs**
   - Mitigation: Comprehensive integration tests
   - Row-level security in database

4. **Scaling to large transaction volumes**
   - Mitigation: Background job queues
   - Horizontal scaling with load balancer

### Business Risks
1. **Regulatory compliance (varies by jurisdiction)**
   - Mitigation: Consult with CPA firms early
   - Implement audit trail and immutability

2. **User trust in AI for financial data**
   - Mitigation: Transparent AI reasoning display
   - Always allow manual override
   - Start with approval-only mode

---

## Conclusion

The foundation is exceptionally strong. All core accounting primitives are implemented in Rust with clean, testable code. The missing pieces are:

1. **AI Integration** - Connecting ChatGPT to the accounting engine
2. **Web UI** - Building the React application
3. **Glue Code** - App server API layer and command handlers

**Estimated Timeline to MVP: 8-12 weeks** with 1-2 full-time developers.

**Estimated Timeline to Production: 4-6 months** including testing, security hardening, and pilot programs.

The architecture is sound, the code quality is high, and the vision is clear. This is a very achievable transformation of Codex CLI into a powerful accounting agent.
