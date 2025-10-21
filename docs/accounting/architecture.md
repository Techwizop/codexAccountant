# Multi-Company Accounting Platform Architecture

## Objectives
- Transform Codex into a modular, multi-tenant accounting platform with AI-assisted posting and human oversight.
- Preserve existing Rust workspace structure while introducing focused accounting crates and protocol extensions.
- Deliver a GUI experience that complements the CLI/TUI and supports drag-and-drop ingestion, review, and reconciliation workflows.
- Enforce rigorous validations aligned with IFRS/GAAP, multi-currency handling, and auditable change history.

## System Overview
The accounting platform layers new domain services on top of the existing Codex core orchestration:

1. **Codex Core** manages conversations, tool dispatch, approval flows, and protocol streaming.
2. **Accounting Domain Services** encapsulate ledger, chart of accounts (COA), tax, and compliance logic exposed through domain-specific tool handlers.
3. **Ingestion & AI Posting Pipeline** consumes documents and statements, classifies transactions, and generates posting proposals subject to validation and human review.
4. **Presentation Layer** spans the existing CLI/TUI plus a new web GUI built on the JSON-RPC app server and TypeScript SDK.
5. **Persistence & Observability** store multi-company ledgers, audit trails, and telemetry using existing tracing hooks augmented for accounting metrics.

## Data Model
| Entity | Purpose | Key Fields |
| --- | --- | --- |
| Company | Top-level tenant scope. | `company_id`, metadata, base currency, fiscal calendar. |
| Chart of Accounts (COA) | Hierarchical structure of accounts per company. | `account_id`, `parent_account_id`, `account_type`, `currency_mode`, `tax_code`. |
| Journal | Logical stream of postings for a company and ledger (GL, AP, AR, cash). | `journal_id`, `ledger_type`, period status, reconciliation flags. |
| Journal Entry | Double-entry transaction referencing a journal and accounts. | `entry_id`, `entry_number`, `status`, timestamps, author, source document linkage. |
| Journal Line | Debit/credit line item. | `line_id`, `account_id`, `amount`, `currency`, `fx_rate`, `dimensions`. |
| Currency Rate | FX conversion snapshot (spot/average). | `pair`, `rate_type`, `effective_at`, `source`. |
| Document | Uploaded artifact feeding ingestion. | `document_id`, type, storage reference, parsed metadata. |
| Audit Event | Immutable log of user/AI actions. | `event_id`, `entity_ref`, action type, timestamps, diff payload. |
| Permission Grant | RBAC binding. | `subject_id`, `role`, `scope`, expiration. |

### Chart of Accounts
- Hierarchical with enforced account types (Asset, Liability, Equity, Revenue, Expense, Off-balance).
- Supports shared templates: base template stored centrally; per-company overrides tracked as deltas.
- Currency mode per account: `FunctionalOnly`, `Transactional`, `MultiCurrency`.
- Dimension support (cost center, project, location) via extensible key-value attributes.

### Journals & Entries
- Each company has ledgers: General (GL), Accounts Payable (AP), Accounts Receivable (AR), Cash, and sub-ledgers as needed.
- Journal entries carry metadata: `origin` (AI, manual, import), `document_ref`, `review_status`, `posting_batch`.
- Lines enforce double-entry (sum debits = sum credits in functional currency) and dimension balancing rules.
- Period management: open, soft-close (requires approval for postings), closed (locked except via adjusting journal entry workflow).

### Multi-Currency Model
- Store both transactional currency amount and functional currency equivalent per line.
- FX handling options: use latest spot rate, user-provided rate, or treasury table. Persist rate source and rounding basis.
- Realized/unrealized gains/losses automatically posted to designated accounts per ledger configuration.

### Tax & Compliance
- Tax codes assigned at account and line levels determine automatic tax lines (e.g., VAT, GST).
- Support jurisdiction-specific validation rules (e.g., US sales tax rounding vs. EU VAT periodic reporting).
- Audit trail captures tax determination steps and overrides.

### Audit & History
- Append-only audit log per entity capturing before/after snapshots.
- Session-level provenance links AI reasoning, tool outputs, and human approvals to resulting journal entries for compliance.

## Ledger APIs
Expose domain services through internal Rust traits and external tool handlers:

| API | Description | Notes |
| --- | --- | --- |
| `LedgerService::create_company` | Provision company, base currency, fiscal calendar. | Seeds COA from template; assigns initial admin role. |
| `LedgerService::upsert_account` | Manage COA nodes with validation (type, hierarchy, currency). | Checks for cycles, duplicate codes. |
| `LedgerService::post_entry` | Validate & persist journal entry with double-entry enforcement and FX conversion. | Returns posting result with audit id; may be `Proposed` until approved. |
| `LedgerService::batch_post` | Efficient posting for imports and AI-approved batches. | Streams results; isolates failures. |
| `LedgerService::reverse_entry` | Create reversing entries within allowed periods. | Ensures reversal tags and audit link. |
| `LedgerService::lock_period` | Transition period state to soft/closed. | Blocks postings unless override role. |
| `LedgerService::revalue_currency` | Generate FX revaluation entries. | Runs per period with rate snapshots. |
| `LedgerService::list_audit_trail` | Query audit events with filters. | Exposes to GUI and reporting. |

APIs prefer asynchronous execution using Tokio, returning structured protocol responses for consistent streaming back to clients.

## Tool Handlers & Domain Services
- **`AccountingLedgerTool`**: wraps ledger APIs for AI workflows. Supports operations like `propose_entry`, `commit_entry`, `void_entry`, and `fetch_account_summary` with strict approval flags.
- **`DocumentIngestionTool`**: orchestrates ingestion pipeline (OCR, classification, data extraction) and attaches parsed data to pending postings.
- **`ComplianceValidationTool`**: runs IFRS/GAAP rule sets, dimension checks, tax compliance; returns remediation hints for AI and reviewers.
- **`ReconciliationTool`**: aligns bank statements, AR/AP schedules, and general ledger; surfaces discrepancies with recommended actions.
- Tools register with `ToolRegistryBuilder` and enforce RBAC scope by verifying session context before execution.

## Protocol Extensions
Introduce new protocol messages under `codex-protocol` to describe accounting workflows:

| Event/Op | Purpose | Payload Highlights |
| --- | --- | --- |
| `Op::AccountingAction` | Structured command from GUI/SDK (e.g., approve posting). | `action_type`, `company_id`, `journal_id`, `payload`. |
| `EventMsg::PostingProposed` | AI or ingestion produced draft entry. | Entry summary, source document, confidence score, blocking issues. |
| `EventMsg::PostingApproved` | Posting committed (auto or human). | Final entry detail, approver, audit id. |
| `EventMsg::ValidationFailed` | Domain checks failed. | Error codes, affected lines, suggested remediations. |
| `EventMsg::ReconciliationStatus` | Streaming reconciliation progress. | Account, period, outstanding variance. |
| `EventMsg::DocumentIngested` | Document processed with extracted data. | Document metadata, ingestion pipeline results. |
| `EventMsg::RoleEscalationRequired` | Sensitive action needing higher privilege approval. | Required role, requested action, reason. |

All events carry `company_id`, `tenant_scope`, and `rbac_context` fields to ensure correct audience and auditability.

## Multi-Tenant & RBAC Context
- Sessions bind to a `TenantContext` struct containing `company_id`, allowed ledgers, locale, and feature flags.
- RBAC model defines roles (Admin, Accountant, Reviewer, Auditor, AI Agent) with scoped permissions (company-wide, ledger-specific, document-level).
- Authorization enforced on:
  - Tool invocation (pre-dispatch check).
  - Protocol-level actions (reject unauthorized `Op::AccountingAction`).
  - GUI routes (middleware verifying JWT/session token claims).
- Support cross-company controllers via composite contexts with explicit consent and logging (e.g., multi-entity consolidations).

## GUI Flows
### Drag-and-Drop Ingestion
1. User drops document into GUI; client uploads to storage and sends `Op::DocumentUpload`.
2. Ingestion tool processes document, emits `EventMsg::DocumentIngested` with parsed data.
3. AI generates posting proposal (`EventMsg::PostingProposed`) and attaches validation summary.
4. Reviewer edits/approves; approval sends `Op::AccountingAction` leading to `EventMsg::PostingApproved`.

### Posting Review & Approval
1. Queue view lists pending postings with risk level indicators.
2. Selecting a posting shows side-by-side document, AI reasoning, and validation results.
3. Reviewer can adjust lines, re-run validation, escalate, or approve.
4. All actions logged via audit events and session transcripts.

### Reconciliation Workflow
1. Import bank or sub-ledger data via ingestion or connectors.
2. Reconciliation tool matches entries, surfaces unmatched items with explanations.
3. Users create adjusting entries or mark as timing differences.
4. Completion emits `EventMsg::ReconciliationStatus` updates and final summary for audit.

### Multi-Company Navigation
- Global nav allows switching companies with context indicator (company name, fiscal period).
- Access limited by RBAC; switching reinitializes session context and reloads ledgers, dashboards, and pending queues.

## Validation Invariants
- **Double-entry balance**: Debits equal credits per entry in functional currency.
- **Account restrictions**: Prevent posting to summary accounts, enforce allowed dimensions, and block closed accounts.
- **Period state**: No postings into closed periods; soft-close requires justification and admin role override.
- **Currency conversions**: Store rate source/time, ensure rounding to configured precision, record realized/unrealized gain/loss entries.
- **Tax compliance**: Auto-generated tax lines must net to expected percentage; overrides require auditor role and justification.
- **Audit immutability**: Posted entries immutable except via reversing or adjusting pathways maintaining linkage.
- **Segregation of duties**: AI cannot self-approve; approvals require human roles with `can_approve_postings` permission.
- **Document linkage**: Every proposed entry ties to at least one document/import event for traceability.

## Proposed Crate Layout
- **Existing Reuse**
  - `codex-core`: orchestration, tool registry integration, session management.
  - `codex-app-server`: transport bridge for GUI/SDK.
  - `codex-protocol`: extend with accounting messages.
  - `codex-tui` / `cli`: serve as admin & ops consoles.
- **New Crates**
  - `codex-ledger`: domain models, persistence traits, posting engine, COA management.
  - `codex-accounting-api`: exposes ledger services as async interfaces consumed by tools and GUI.
  - `codex-ingest`: document pipeline, connectors, classification models.
  - `codex-compliance`: IFRS/GAAP validation rules, tax logic.
  - `codex-permissions`: RBAC scope definitions, policy evaluation helpers.
  - `codex-gui` (TypeScript workspace): web client using SDK, packaged separately.
  - `codex-reporting` (future): statutory & management reporting utilities.

## Milestones & Deliverables
| Milestone | Scope | Key Deliverables | Exit Criteria |
| --- | --- | --- | --- |
| M0 – Foundations | Multi-tenant scaffolding, protocol extensions baseline. | `TenantContext` wiring, initial accounting protocol messages, design review sign-off. | Tests proving session scoping; architecture doc approved. |
| M1 – Ledger Core | COA, posting engine, audit log MVP. | `codex-ledger` crate with unit tests, double-entry enforcement, COA editor CLI/TUI support. | Ability to post manual entries with validation. |
| M2 – Ingestion & AI Posting | Document pipeline, posting proposals, validation engine. | `codex-ingest`, `codex-compliance`, tool handlers integrated with Codex core, pending posting queue UI. | AI can propose entries; reviewers approve with audit trail. |
| M3 – GUI Launch | Web GUI for multi-company workflows. | `codex-gui` prototype with ingestion, review, approvals, context switching. | Pilot customers operate fully via GUI; parity with CLI essentials. |
| M4 – Advanced Financial Ops | Reconciliations, multi-currency revaluation, tax filing support. | Reconciliation tool, FX revaluation jobs, tax reporting exports. | Period close workflows executed end-to-end. |
| M5 – Compliance Hardening | External integrations, observability, certifications. | ERP connectors, SOC-ready logging, retention policies, penetration test remediation. | Passed compliance audit checklist; production readiness. |

## Open Questions & TODOs
- Finalize persistence choice (PostgreSQL vs. existing storage abstractions) and migration strategy.
- Determine AI model responsibilities vs. deterministic rule engines for tax and FX handling.
- Decide on GUI technology stack (React + Vite + Tailwind?) and deployment pipeline integration.
- Establish residency and data retention requirements per jurisdiction.
- Define strategy for extensible reporting (custom financial statements, KPIs).

## Current Implementation Baseline
- `codex-ledger` provides domain types plus an in-memory `LedgerService`; no persistence layer is wired in yet.
- The app server exposes JSON-RPC hooks guarded behind the optional `ledger` feature flag and uses the in-memory service when `CODEX_LEDGER_IN_MEMORY=1`, now including FX revaluation and audit trail listings wired through the shared `codex-accounting-api` facade.
- CLI provides `codex ledger` subcommands to seed demo data and list companies, accounts, and recent journal entries; the TUI adds an F6 ledger overlay that summarizes the same dataset for quick reference.
- Docs (this file) capture the desired end state but lack concrete sequencing for the CLI, toolchain, and GUI convergence.

### Telemetry Persistence
- Demo commands and the TUI use `AccountingTelemetry::persistent_from_env()` so counters accumulate under `CODEX_HOME/accounting/telemetry.json`.
- Reconciliation JSON replies include `telemetry_path`, transaction dedupe metadata, and candidate write-off references so operators can script around the persisted state.
- CLI text output now prints a `Telemetry file:` line, and streaming helpers (`codex tenancy list --stream-reconciliation`) echo the same path so operators know where telemetry lives.
- To reset counters, delete the JSON file before rerunning commands; corrupt files fall back to defaults with a warning and are overwritten on the next update.

### Go-Live Checklist
- The `codex ledger go-live-checklist` command now highlights the `codex ledger entries --format json` export path, flags monitoring TODOs (metrics dashboards plus pager rotation), and emits a telemetry reset reminder pointing back to `<CODEX_HOME>/accounting/telemetry.json` so operators clear demo counters between dry runs.

## Transformation Blueprint
1. **Ledger-first instrumentation**
   - Ship an embedded `LedgerOrchestrator` that wraps `LedgerService` implementations, tracks GPT-5 reasoning traces, and enforces RBAC gates.
   - Replace ad-hoc conversions in the app server with dedicated mappers under a forthcoming `codex-accounting-api` crate so the CLI/TUI and GUI share the same facade.
2. **Agent workflow evolution**
   - Introduce accounting-specific toolchains inside `codex-core` (`Op::AccountingAction`, reconciliation intents, posting proposal pipelines).
   - Extend conversation templates so GPT-5 acts as the accountant brain: prime with ledger context, define guardrails, and persist narrative alongside postings.
3. **Experience layers**
   - TUI: add company switcher, posting queue browser, and approval flows using ratatui helpers; seed snapshot coverage for the new panes.
   - GUI: bootstrap a React + Vite app under `apps/codex-gui` that talks to the app server protocol; focus first on authentication, company selection, and posting review.
4. **Autonomy & compliance**
   - Automate ingestion via `codex-ingest` jobs with retryable states; integrate validation failures into GPT-5 follow-up prompts.
   - Record audit evidence by default (AI rationale, human overrides, tax outcomes) and surface in both CLI and GUI.

## Immediate Focus Areas
- Stand up `codex-accounting-api` with request/response DTOs shared across transports; migrate the app server to it while keeping the coding agent path intact.
- Define GPT-5 prompt scaffolding plus safety rails in `docs/accounting/prompting.md` (new) so agent reasoning stays auditable.
- Prototype the TUI posting queue pane guarded behind a feature flag; once snapshots stabilize, promote to default.
- Land a minimal GUI shell that loads companies, renders GPT-5 task summaries, and deep links into posting approvals.
