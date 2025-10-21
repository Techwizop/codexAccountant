# Development Plan for AI Coding Agents
## Codex Accounting - Complete Task List

**Mission**: Transform Codex CLI into autonomous accounting software  
**Status**: Backend 80% | **Tasks**: 250 | **Timeline**: 8-12 weeks

**Reference**: See DEVELOPMENT_ROADMAP.md, QUICK_START_GUIDE.md for detailed specs

---

## Task Format
Each task: **Task N**: Description [Path] [~Lines] [Priority] [Dependencies]
- ✓ **Accept**: Acceptance criteria

**Priorities**: P0=Critical, P1=High, P2=Medium, P3=Nice-to-have

---

## PHASE 1: AI FOUNDATION (Tasks 1-30, Weeks 1-3)

### Core Tools (Week 1)
**Task 1**: Create accounting tools module [codex-rs/core/src/tools/accounting.rs] [~600] [P0]
- Implement 7 structs: CreateCompanyTool, ListCompaniesTool, UpsertAccountTool, ListAccountsTool, PostJournalEntryTool, ListEntriesTool, GetCompanyContextTool
- Each: parse args → call LedgerFacade → return JSON
- ✓ All tools implement ToolHandler, compile clean

**Task 2**: Register tools [core/src/tools/registry.rs] [~30] [P0] [Dep: 1]
- Add mod accounting, register under feature flag
- ✓ Tools callable, feature works

**Task 3**: Define function schemas [core/src/tools/definitions.rs] [~200] [P0] [Dep: 1]
- Create FunctionDefinition for each tool with JSON schema
- ✓ ChatGPT can call all tools

**Task 4**: Add feature to Cargo.toml [~5] [P0]
- ✓ cargo build --features ledger works

**Task 5**: Write tool tests [~300] [P1] [Dep: 1-4]
- Test each with mocks, use pretty_assertions
- ✓ All pass, >80% coverage

### Document Agent (Week 1-2)
**Task 6**: Create accounting module [core/src/accounting/mod.rs] [~20] [P0]
- ✓ Module structure compiles

**Task 7**: Document agent [accounting/document_agent.rs] [~500] [P0]
- Struct: DocumentAgent, methods: process_document, extract_invoice_data, suggest_journal_entry
- Validate debits == credits
- ✓ Full flow works, balanced entries

**Task 8**: Define data structures [~200] [P0] [Dep: 7]
- InvoiceData, LineItem, JournalEntrySuggestion, SuggestedLine
- ✓ All Serialize + Deserialize

**Task 9**: ChatGPT prompts [~150] [P0] [Dep: 7]
- Extraction + suggestion templates with JSON schemas
- ✓ Returns valid JSON

**Task 10**: Error handling [~100] [P0] [Dep: 7-9]
- Handle malformed JSON, validate fields
- ✓ Graceful failures

**Task 11**: Agent tests [~400] [P1] [Dep: 7-10]
- Test extraction, suggestion, balance, end-to-end
- ✓ 5+ tests pass

### Posting Agent (Week 2)
**Task 12**: Posting agent [accounting/posting_agent.rs] [~400] [P0] [Dep: 7]
- Struct: PostingAgent, methods: run (loop), handle_document
- Flow: extract → policy → post/queue
- ✓ Runs continuously, routes correctly

**Task 13**: Policy integration [~150] [P0] [Dep: 12]
- Create PostingProposal, call policy_engine.evaluate
- ✓ Routes on PolicyDecision

**Task 14**: Approval integration [~150] [P0] [Dep: 12]
- Create ApprovalRequest, call approvals_service.enqueue
- ✓ Creates with context

**Task 15**: Post to ledger [~100] [P0] [Dep: 12]
- Convert suggestion to PostEntryRequest, call facade
- ✓ Posts successfully

**Task 16**: Processing queue [~250] [P0] [Dep: 12]
- Track stages, persist status
- ✓ Tracks stages, concurrent

**Task 17**: Retry logic [~100] [P1] [Dep: 12-16]
- Retry OCR/ChatGPT 3x with backoff
- ✓ Retries work

**Task 18**: Telemetry [~100] [P1] [Dep: 12-17]
- Log actions, track times, count outcomes
- ✓ Metrics observable

**Task 19**: Integration tests [~400] [P1] [Dep: 12-18]
- Full flow with mocks
- ✓ End-to-end passes

### Context & Learning (Week 2-3)
**Task 20**: Context builder [accounting/context.rs] [~300] [P1]
- Build system prompt with chart, transactions, mappings, policy
- ✓ Context enhances responses

**Task 21**: Vendor mappings [~150] [P1] [Dep: 20]
- Store vendor → account, learn from corrections
- ✓ Mappings persist

**Task 22**: Pattern recognition [~200] [P1] [Dep: 20]
- Detect recurring, suggest rules
- ✓ Identifies patterns

**Task 23**: Upload handler [accounting/upload_handler.rs] [~200] [P1]
- Call doc-ingest, return upload ID
- ✓ Uploads PDFs

**Task 24**: OCR integration [accounting/ocr_integration.rs] [~150] [P1]
- Call ocr service
- ✓ Returns text + confidence

**Task 25**: Document links [~100] [P1]
- Link docs to entries
- ✓ Entries link to docs

**Task 26**: CLI commands [cli/src/commands/accounting.rs] [~600] [P1]
- Implement: company create/list, upload, ledger show, approvals list/approve, reconcile, reports
- ✓ All commands work

**Task 27-30**: Test CLI [~200] [P2] [Dep: 26]
- ✓ Commands produce expected output

---

## PHASE 2: APP SERVER API (Tasks 31-60, Weeks 3-4)

### Protocol (Week 3)
**Task 31**: Company types [app-server-protocol/src/lib.rs] [~150] [P0]
- CreateCompanyParams/Response, ListCompaniesParams/Response, CompanyInfo, use #[derive(TS)]
- ✓ Types compile, TS bindings

**Task 32**: Account types [~150] [P0]
- UpsertAccountParams/Response, ListAccountsParams/Response, AccountInfo
- ✓ Match ledger service

**Task 33**: Entry types [~200] [P0]
- PostEntryParams/Response, ListEntriesParams/Response, EntryLine, JournalEntryInfo
- ✓ Support all operations

**Task 34**: Approval types [~150] [P0]
- ListApprovalsParams/Response, ApproveEntryParams/Response, ApprovalInfo
- ✓ Match approval service

**Task 35**: Reconcile types [~200] [P0]
- StartReconciliationParams/Response, AcceptMatchParams/Response, MatchCandidateInfo
- ✓ Match reconcile service

**Task 36**: Upload types [~100] [P0]
- UploadDocumentParams/Response, ListDocumentsParams/Response, DocumentInfo
- ✓ Signed URL flow

**Task 37**: Report types [~150] [P0]
- GenerateTrialBalanceParams, TrialBalanceResponse, GeneratePLParams, PLResponse, etc.
- ✓ Complete

**Task 38**: Generate TS bindings [~0] [P0] [Dep: 31-37]
- Run cargo build
- ✓ bindings.ts generated

### Handlers (Week 3-4)
**Task 39**: Main handler [app-server/src/accounting_handlers.rs] [~800] [P0]
- handle_accounting_method router
- ✓ Routes all methods

**Task 40**: Wire to processor [app-server/src/message_processor.rs] [~50] [P0] [Dep: 39]
- if method.starts_with("accounting/")
- ✓ Routes correctly

**Task 41-55**: Individual handlers [~50-80 each] [P0] [Dep: 39-40]
- handle_create_company, handle_list_companies, handle_upsert_account, handle_list_accounts, handle_post_entry, handle_list_entries, handle_list_approvals, handle_approve_entry, handle_reject_entry, handle_start_reconciliation, handle_accept_match, handle_upload_document, handle_list_documents, handle_generate_trial_balance, handle_generate_pl
- Each: deserialize → validate → call facade → serialize
- ✓ Each works correctly

### Testing (Week 4)
**Task 56**: API tests [app-server/tests/accounting_api_test.rs] [~600] [P1]
- Test all JSON-RPC methods
- ✓ All tested

**Task 57**: Validation [~200] [P1]
- Validate params, return errors
- ✓ Invalid rejected

**Task 58**: WebSocket tests [~150] [P1]
- Test streaming updates
- ✓ Updates work

**Task 59-60**: Snapshot tests [~200] [P2]
- ✓ Responses match format

---

## PHASE 3: WEB UI (Tasks 61-150, Weeks 4-8)

### Setup (Week 4)
**Task 61**: Init Vite+React [apps/codex-gui/]
- pnpm create vite . --template react-ts
- ✓ Dev server runs

**Task 62**: Install deps
- @tanstack/react-query, react-router, zustand, tailwindcss, lucide-react, recharts, etc.
- ✓ All install

**Task 63**: Tailwind config [~50] [P0]
- ✓ Classes work

**Task 64**: TS config [~50] [P0]
- ✓ Imports resolve

**Task 65**: RPC client [src/lib/rpc-client.ts] [~300] [P0]
- WebSocket, type-safe methods
- ✓ Connects, messages work

**Task 66**: React Query setup [~100] [P0]
- ✓ Queries work

**Task 67**: Router [src/router.tsx] [~150] [P0]
- Routes for all pages
- ✓ Navigation works

**Task 68**: Zustand store [src/store/app-store.ts] [~200] [P0]
- currentCompanyId, user, notifications
- ✓ State persists

### Layout (Week 5)
**Task 69**: App shell [src/components/layout/AppShell.tsx] [~200] [P0]
- Header, sidebar, main, responsive
- ✓ Renders on all devices

**Task 70**: Sidebar [layout/Sidebar.tsx] [~150] [P0]
- Nav links, active state
- ✓ All routes accessible

**Task 71**: Header [layout/Header.tsx] [~200] [P0]
- Company dropdown, user menu, notifications
- ✓ Can switch companies

**Task 72**: Company selector [company/CompanySelector.tsx] [~250] [P0]
- Search, recent, create dialog
- ✓ Lists, creates

**Task 73**: Notification center [notifications/NotificationCenter.tsx] [~200] [P0]
- Toasts, list
- ✓ Shows notifications

**Task 74**: Dark mode [~100] [P2]
- ✓ Theme toggles

### Core Pages (Week 5-6)
**Task 75**: Dashboard [src/pages/Dashboard.tsx] [~400] [P0]
- Pending, unreconciled, activity, stats
- ✓ Real data

**Task 76**: Upload center [pages/UploadCenter.tsx] [~500] [P0]
- Drag-drop, progress, list
- ✓ Uploads work

**Task 77**: File upload [upload/FileUpload.tsx] [~300] [P0]
- Drag-drop, multiple, progress
- ✓ Upload + progress

**Task 78**: Document list [upload/DocumentList.tsx] [~250] [P0]
- Table, filters, actions
- ✓ Lists docs

**Task 79**: Doc preview [upload/DocumentPreview.tsx] [~200] [P1]
- PDF/image viewer
- ✓ Preview renders

**Task 80**: Approvals page [pages/Approvals.tsx] [~600] [P0]
- List, filters, detail
- ✓ Full flow

**Task 81**: Approval list [approvals/ApprovalList.tsx] [~350] [P0]
- Table, sort, filter
- ✓ Works

**Task 82**: Approval detail [approvals/ApprovalDetail.tsx] [~400] [P0]
- Doc, reasoning, editor, actions
- ✓ Can edit/approve

**Task 83**: Entry editor [journal/EntryEditor.tsx] [~450] [P0]
- Lines table, balance validation
- ✓ Edits, validates

**Task 84**: Transactions page [pages/Transactions.tsx] [~500] [P0]
- List, filters, drawer
- ✓ Lists entries

**Task 85**: Transaction list [journal/TransactionList.tsx] [~350] [P0]
- Table, pagination
- ✓ Shows entries

**Task 86**: Transaction detail [journal/TransactionDetail.tsx] [~300] [P1]
- Lines, audit, reverse
- ✓ Complete details

### Reconciliation (Week 6)
**Task 87**: Reconcile page [pages/Reconciliation.tsx] [~700] [P1]
- Upload, matches
- ✓ Full flow

**Task 88**: Bank upload [reconcile/BankUpload.tsx] [~250] [P1]
- CSV/OFX, preview
- ✓ Parses

**Task 89**: Match candidates [reconcile/MatchCandidates.tsx] [~400] [P1]
- Comparison, scores, actions
- ✓ Matches work

### Reports (Week 6-7)
**Task 90**: Reports page [pages/Reports.tsx] [~400] [P1]
- Selector, params, export
- ✓ Generates

**Task 91**: Trial balance [reports/TrialBalance.tsx] [~350] [P1]
- Table, export
- ✓ Correct

**Task 92**: P&L [reports/ProfitLoss.tsx] [~400] [P1]
- Revenue/expense, net
- ✓ Calculates

**Task 93**: Balance sheet [reports/BalanceSheet.tsx] [~400] [P1]
- A/L/E
- ✓ A=L+E

### Settings & Chat (Week 7)
**Task 94**: Settings page [pages/Settings.tsx] [~500] [P2]
- ✓ Persists

**Task 95**: Chart editor [settings/ChartEditor.tsx] [~500] [P2]
- ✓ Manages accounts

**Task 96**: Policy config [settings/PolicyConfig.tsx] [~400] [P2]
- ✓ Saves

**Task 97**: Chat dock [chat/ChatDock.tsx] [~500] [P1]
- ✓ Chat works

**Task 98**: Chat messages [chat/ChatMessage.tsx] [~200] [P1]
- ✓ Renders

**Task 99**: Context integration [~150] [P1]
- ✓ Knows context

**Task 100**: Suggested prompts [~150] [P2]
- ✓ Updates

### Polish (Week 8)
**Task 101-110**: Loading, errors, animations, tooltips, keyboard, a11y, responsive, empty states, confirms, search [~50-150 each] [P2]
- ✓ UI polished

**Task 111-120**: Playwright E2E [~1000] [P1]
- Test critical flows
- ✓ Tests pass

**Task 121-130**: Vitest component tests [~800] [P1]
- ✓ Tests pass

**Task 131-140**: Storybook [~500] [P3]
- ✓ Documented

**Task 141-150**: Performance [~400] [P2]
- Virtual scroll, memoization, splitting, optimization
- ✓ Lighthouse >90

---

## PHASE 4: CLI/TUI (Tasks 151-170, Weeks 5-8)

**Task 151-160**: CLI implementations [~60-80 each] [P1]
- Detailed command implementations
- ✓ Each works

**Task 161-170**: TUI screens [~150-200 each] [P2]
- Company, upload, approvals, transactions, reports
- ✓ TUI functional

---

## PHASE 5: AI ENHANCEMENTS (Tasks 171-200, Weeks 8-9)

**Task 171**: Learning system [learning.rs] [~400] [P1]
- ✓ Learns from feedback

**Task 172**: Patterns [~350] [P1]
- ✓ Identifies patterns

**Task 173**: Metrics [~250] [P1]
- ✓ Dashboard

**Task 174**: Vendor fuzzy match [~200] [P1]
- ✓ Matches similar

**Task 175**: Tax suggestions [~200] [P1]
- ✓ Correct codes

**Task 176**: Accrual detection [~250] [P1]
- ✓ Suggests adjustments

**Task 177**: Period-end assistant [~300] [P1]
- ✓ Guides close

**Task 178**: Variance explanations [~250] [P1]
- ✓ Narratives

**Task 179**: Mgmt reports [~300] [P1]
- ✓ Professional

**Task 180-190**: Additional AI features [~150-300 each] [P2]
- Duplicates, anomalies, forecasting, cash flow, budget, multi-currency, consolidation, custom reports, NL queries
- ✓ Each functional

**Task 191-200**: AI testing [~100-200 each] [P1]
- ✓ >85% accuracy

---

## PHASE 6: PRODUCTION (Tasks 201-250, Weeks 9-12)

### Database (Week 9)
**Task 201**: PostgreSQL schema [migrations/001_accounting.sql] [~500] [P0]
- ✓ Matches models

**Task 202**: Migration tooling [~50] [P0]
- ✓ Runs

**Task 203**: Ledger persistence [codex-ledger/src/postgres.rs] [~800] [P0]
- ✓ DB works

**Task 204-208**: Other service persistence [~300-400 each] [P0]
- Approvals, reconcile, policy, docs, audit
- ✓ Each in DB

**Task 209**: Connection pooling [~100] [P0]
- ✓ Efficient

**Task 210**: Transactions [~150] [P0]
- ✓ ACID

### Storage & Security (Week 10-11)
**Task 211**: S3 storage [~300] [P0]
- ✓ Docs in S3

**Task 212**: Thumbnails [~200] [P0]
- ✓ Generated

**Task 213**: Doc links [~100] [P0]
- ✓ Linked

**Task 214**: RBAC [~400] [P0]
- ✓ Enforced

**Task 215**: Auth middleware [~200] [P0]
- ✓ Required

**Task 216**: Tenant isolation [~150] [P0]
- ✓ No leaks

**Task 217**: Rate limiting [~150] [P1]
- ✓ Enforced

**Task 218**: Encryption [~200] [P0]
- ✓ Encrypted

**Task 219**: TLS [~50] [P0]
- ✓ Required

**Task 220**: Security audit [~200] [P0]
- ✓ No critical issues

### Compliance (Week 11)
**Task 221**: Immutable audit [~300] [P1]
- ✓ Tamper-proof

**Task 222**: Audit reports [~250] [P1]
- ✓ Complete trail

**Task 223**: Retention [~150] [P1]
- ✓ Works

**Task 224**: Exports [~200] [P1]
- ✓ QB/Xero/CSV

### Performance (Week 11-12)
**Task 225**: DB indexes [~100] [P1]
- ✓ Improved

**Task 226**: Caching [~250] [P1]
- ✓ >80% hit rate

**Task 227**: Job queue [~300] [P1]
- ✓ Reliable

**Task 228**: Horizontal scaling [~200] [P1]
- ✓ Multi-instance

**Task 229**: Performance testing [~300] [P1]
- ✓ Handles 100 users, <2s

**Task 230**: Large datasets [~200] [P1]
- ✓ Handles 10K+ txns

### Integrations (Week 12)
**Task 231-240**: Bank feeds, payments, accounting exports, email, webhooks, API docs, monitoring, backups, deployment, health checks [~200-400 each] [P2]
- ✓ Each works

### Final (Week 12)
**Task 241-250**: Beta testing, bug fixes, docs, training, pilot launch, monitoring, support, optimization, polish, release [~100-300 each] [P1-P2]
- ✓ Production ready

---

## Success Metrics

**Week 2**: AI tools work, can extract invoices  
**Week 4**: API complete, UI scaffold  
**Week 6**: Full upload→approval→post flow  
**Week 8**: Reconciliation, reports, chat  
**Week 10**: Database, security  
**Week 12**: Production ready, pilot launch

**Target**: 80%+ extraction accuracy, 70%+ auto-match, <5s processing, 99.9% uptime
