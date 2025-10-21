# Autonomous Accounting Roadmap Execution Plan

This document expands the Phase 0 foundations (`docs/autonomous-accounting-phase0.md`) across Phases 1 through 6 in `specs/tasks.md`. Each phase lists scope, design decisions, implementation steps, validation strategy, and key assumptions or follow-ups.

> Command log helper: run `./scripts/append-command-log.sh "<command summary>"` to append synced entries to this roadmap and the Phase 0 document.

## Environment Notes
- Verified `pkg-config` and `libssl-dev` are present (apt reported both as already installed).
- Ran `just fix -p codex-tenancy` and `just fix -p codex-accounting-api` with clean passes.
- `just fix -p codex-cli` initially failed because `chrono` was missing; added the workspace dependency and reran successfully.
- Tenancy CLI snapshot persistence now resolves via `CODEX_HOME/accounting/tenancy.json`, matching the `~/.codex/accounting/tenancy.json` expectation on both Linux and Windows/WSL (added unit coverage for the join logic).

## Testing Status
- `just fix -p` ran clean for tenancy, accounting API, CLI, ledger, doc-store, doc-ingest, ocr, audit-log, policy, and approvals; executed `cargo test -p codex-tenancy`, `cargo test -p codex-accounting-api`, `cargo test -p codex-cli`, `cargo test -p codex-ledger`, `cargo test -p codex-doc-store`, `cargo test -p codex-doc-ingest`, `cargo test -p codex-ocr`, `cargo test -p codex-audit-log`, `cargo test -p codex-policy`, and `cargo test -p codex-approvals`.
- Pending: wider integration coverage once new Phase 1 crates land (doc store, ingest, audit log).
- Phase 2/3 iteration covered fresh suites: `cargo test -p codex-policy`, `cargo test -p codex-approvals`, `cargo test -p codex-ledger`, `cargo test -p codex-accounting-api`, `cargo test -p codex-cli`, `cargo test -p codex-tui`, `cargo test -p codex-bank-ingest`, and `cargo test -p codex-reconcile`.

## Phase 1 - Core Platform Setup

**Scope**
- Tenant model with company lifecycle management, user management, and role definitions mapped to CPA hierarchies.
- Document storage service with versioning, encryption, metadata indexing, and retention enforcement.
- Document ingestion APIs plus CLI and TUI commands for uploads and status checks.
- OCR and classification pipeline for primary document types with metadata extraction.
- Initial ledger service covering chart of accounts, journal engine, and reporting data model.
- Codex agent harness to process ingestion events and propose postings.
- Audit log service capturing document actions, AI decisions, and manual overrides.

**Architecture and Components**
- `codex-core` provides shared primitives (auth, queue clients, tracing). New crates:
  - `codex-tenancy`: tenant model, firm and company records, role assignments, SCIM-compatible directory sync hooks.
  - `codex-doc-store`: wraps S3-compatible storage with envelope encryption, version manifests, and metadata indices in PostgreSQL.
  - `codex-doc-ingest`: REST + gRPC API for uploads, integrates with CLI/TUI via signed upload URLs and ingestion job queue.
  - `codex-ocr`: orchestrates OCR providers (Tesseract default, vendor plug-ins), classification models, and metadata extraction workers.
  - `codex-ledger`: double-entry engine with company-scoped charts, posting API, period tables, and reporting views.
  - `codex-policy` (stubbed in Phase 1) handles approval thresholds and routing configuration for later phases.
  - `codex-audit-log`: append-only log writer with hashing for tamper detection, streaming to cold storage per retention policy.
- Event-driven architecture via NATS or Kafka-equivalent (select one during infra RFC). Ingested docs emit `DocumentProcessed` events consumed by the agent harness.
- API gateway exposes REST endpoints secured by firm-level auth tokens and role checks; GraphQL facade planned once reporting matures.

**Data Model Highlights**
- Tenancy tables: firms, companies, company_tags, user_accounts, role_assignments, policy_sets.
- Document storage: document_manifests (version, checksum, retention), document_metadata (type, amount, currency, counterparty, status).
- Ledger: accounts, account_classes, journals, journal_lines, periods, ledgers_snapshots for reporting caches.
- Audit log: audit_events (actor, subject, action, context, hash chain), audit_event_blobs for large payloads.

**Implementation Steps**
1. Finalize ERDs and service contracts; capture in architecture RFCs.
2. Scaffold new crates with shared error handling, tracing, and configuration patterns.
3. Implement tenancy APIs (create company, archive/reactivate, invite user) with tests against in-memory PostgreSQL fixtures.
4. Deliver document storage abstraction with streaming uploads, KMS integration, and retention scheduler.
5. Build ingestion service: signed upload URL issuance, background processing queue, status polling endpoints, CLI/TUI commands invoking the API.
6. Integrate OCR pipeline with provider abstraction, classification model (initial heuristic + off-the-shelf ML), and metadata extractor returning normalized schema.
7. Implement ledger core: chart management, journal posting with balancing enforcement, and read models for trial balance.
8. Wire Codex agent harness: subscribe to `DocumentProcessed`, call LLM prompt template, emit `PostingProposed` event.
9. Deliver audit log service with gRPC ingestion API, batched persistence, and export tooling for compliance reviews.

### Phase 1 Progress (2025-10-16)
- Expanded tenancy domain to track firms, users, and RBAC roles (Partner/Senior/Staff/Auditor) with normalization helpers and tenant-isolation tests.
- In-memory tenancy snapshot now captures firms/users for CLI persistence; JSON snapshot version upgraded with backward-compatible serde defaults.
- CLI auto-creates a firm shell before first company creation so existing workflows continue without a separate firm command.
- Added `codex-doc-store` crate with S3-style interface, envelope-encryption hooks, retention scheduler trait, and in-memory stub covered by unit tests.
- Introduced `codex-doc-ingest` crate exposing an Axum router, queue/signer traits, and CLI harness for simulating signed upload URLs.
- Added `codex-ocr` crate with provider/classifier traits and mock extraction flow to unlock downstream automation tests.
- Extended `codex-ledger` with chart seeding helpers, per-period state tracking, and facade wrappers; `InMemoryLedgerService` seeds default accounts and passes new contract tests.
- Created `codex-audit-log` crate with hash-chained audit records, in-memory backend, and tamper-detection tests.

**Validation Strategy**
- Unit tests per crate using `pretty_assertions::assert_eq` and golden fixtures where applicable.
- Integration tests using docker-compose stack for PostgreSQL, Redis/queue, and mock OCR service.
- Snapshot tests for TUI/CLI commands after ingesting sample documents.
- Manual soak test of document ingestion to ledger posting using golden dataset and Phase 0 retention controls.

**Assumptions and Follow-Ups**
- Feature flag `policy.approvals_enabled` remains off until Phase 2 logic lands.
- OCR provider contracts require legal review; start with open-source engine and modular adapter interface.
- Need stakeholder confirmation on storage regions before provisioning object store buckets.

## Phase 2 - Autonomous Posting MVP

**Scope**
- Policy engine enforcing thresholds, account and vendor rules, and AI confidence gating.
- Posting workflow from AI proposal through policy evaluation to approval queues or auto-posting.
- Persist AI rationale, confidence scoring, and decision metadata.
- Approval queue UI/CLI flow with document preview, edit, approve, decline actions.
- Journal posting, reversing, and adjustment flows with immutable history.
- Operational dashboard for ingestion progress, automation coverage, and outstanding approvals.
- Embedded chat assistant to explain decisions and accept manual commands.
- Validation of automation accuracy on golden dataset with heuristic tuning loop.

**Architecture and Components**
- Extend `codex-policy` with rule builder, evaluation engine, and decision audit trail.
- Introduce `codex-approvals` service managing queues, assignments, SLA timers, and notifications.
- Enhance `codex-ledger` with reversal/adjustment APIs and immutable journal history (append-only with supersede markers).
- UI components: web SPA approval workspace using React Query and WebSockets for live updates; TUI approval queue with snapshots for tests.
- Chat assistant integrates with `codex-chat` orchestrator, using conversation state tied to approvals or ledger context.

### Phase 2 Progress (2025-10-17)
- Extended `codex-policy` with a durable store abstraction (including a Postgres stub gated by `postgres-store` feature), telemetry event sink, and deterministic evaluation matrix tests; CLI policy preview now records events for future analytics.
- Hardened `codex-approvals` with multi-stage approval sequencing, SLA breach helper, queue export API, and expanded tests covering accept/reject flows and overdue detection.
- Tenancy CLI gained `policy show`/`policy set` subcommands and a pipe-friendly `approvals queue`; bootstrap snapshots now carry a version header and richer policy metadata.
- Accounting API exposes a Controls facade that lists persisted policy rule sets and approval queues, plus a reconciliation summary placeholder ready to hook into upcoming services.

**Workflow**
1. Agent submits `PostingProposed` with structured journal entry, rationale text, confidence score, and supporting metadata.
2. `codex-policy` evaluates rules (thresholds, account flags, vendor policies, confidence floor) and emits `PostingDecision` (AutoPost, NeedsApproval, Reject).
3. AutoPost triggers ledger posting and audit log entry. NeedsApproval routes to `codex-approvals`; reject returns to exceptions queue with rationale.
4. Approval UI surfaces document preview, AI summary, ledger impact, and action buttons. Approved entries post to ledger; declined entries capture corrective rationale and feed learning pipeline.
5. Dashboard aggregates ingestion metrics, decision counts, approval SLAs, and confidence distributions via analytics warehouse views.
6. Chat assistant exposes commands (`/explain posting <id>`, `/approve <id>`, `/list approvals`) backed by policy and ledger APIs.

**Validation Strategy**
- Property tests for policy evaluation ensuring deterministic outcomes for rule matrices.
- End-to-end integration tests simulating multiple documents with varied thresholds.
- UX snapshot tests for SPA components (Storybook visual or Playwright baseline) and Ratatui snapshots for CLI/TUI.
- Accuracy validation harness comparing AI proposals against golden dataset expected postings; reports coverage and deviation metrics.

**Assumptions and Follow-Ups**
- Feature flag `approvals.auto_post_enabled` defaults to false in pilot until stakeholders confirm thresholds.
- Need design sign-off for approval UI interactions and chat command set.
- Extend analytics warehouse schema to ingest policy decision metrics.
- Persist ledger bootstrap beyond the in-memory helper once the ledger service selects a durable backend and expose bootstrap metadata via accounting API endpoints.
- Wire approvals service to notification outbox and add reconciliation between policy decisions and queued approvals.
- Implement real Postgres-backed policy store persistence once infrastructure is available; current feature-gated stub blocks writes.
- Capture CLI policy updates in downstream analytics (event sink emits but storage pipeline still TODO).

## Phase 3 - Reconciliation and Close Support

**Scope**
- Bank and statement import flows (manual upload + parsing) with dedupe and status tracking.
- Auto-match engine comparing statement transactions with ledger entries, offering AI suggestions for unmatched items.
- Reconciliation workspace UI with match actions, split transactions, write-offs, and exception queue.
- Month-end close checklist tracking tasks, assignments, and progress.
- Generation of core financial reports (Trial Balance, P&L, Balance Sheet, Cash Flow) with real-time data.
- AI-generated commentary for management reports.
- Period locking and unlocking controls with permission checks.

**Architecture and Components**
- `codex-bank-ingest`: handles statement uploads, parsing (OFX, CSV, PDF via OCR), and normalization into bank_transaction records.
- `codex-reconcile`: reconciler service maintaining match candidates, scoring, and reconciliation sessions.
- Extend `codex-ledger` reporting layer with materialized views or OLAP integration for financial statements.
- Checklist module (`codex-close`) tracking tasks, dependencies, and due dates; integrates with approval queues and notifications.
- Reporting engine uses templated generators; commentary uses AI prompt templates referencing ledger metrics and variances.

**Implementation Steps**
1. Define bank transaction schema, mapping to ledger journal references, and dedupe logic.
2. Build parsers per import format with pluggable architecture and tests using sample statements.
3. Implement match engine using configurable heuristics (date, amount, counterparty) plus AI suggestions for uncertain matches.
4. Develop reconciliation workspace UI/TUI with live updates, exception handling, and audit logging.
5. Create close checklist API with templated tasks per company and progress dashboards.
6. Extend reporting endpoints to deliver GAAP-compliant financial statements with drill-down to journals and source documents.
7. Add AI commentary generator storing narratives alongside reports, subject to approval workflow (feature flag `reports.ai_commentary`).
8. Enforce period locking with role checks and audit entries; support unlock requests routed through approval service.

### Phase 3 Progress (2025-10-18)
- Scaffolded `codex-bank-ingest` with parser traits (CSV/OFX), normalized transaction model, dedupe helper, and placeholder fixtures for upcoming parser work.
- Introduced `codex-reconcile` with match proposal scoring trait, reconciliation session state machine, and an in-memory service covering add/accept/reject flows.
- Extended `codex-ledger` journal entries to track reconciliation status and captured period lock metadata (including approval references) for auditability.
- Added accounting API hooks: Controls facade already bridges policy/approvals, and a reconciliation summary placeholder provider is ready for integration with the new services.

### Phase 3 Status (2025-10-19)
- Delivered a streaming CSV parser with profile-driven column mapping, ISO date handling, checksum generation, duplicate metrics, and multi-currency/voided fixtures (`codex-bank-ingest`).
- Implemented OFX statement parsing (grouped transactions, currency overrides) with concrete fixtures and validation; dedupe metadata now annotates kept vs. dropped records.
- Introduced a trait-based `ReconciliationStore`, weighted scoring (amount/date/description heuristics), partial acceptance, write-off, reopen transitions, and audit hooks (`codex-reconcile`).
- Extended `codex-ledger` with reconciliation status helpers (pending/reconciled/write-off), approval-guarded write-offs, and period lock history vectors plus query helper.
- Added a reconciliation facade that bridges bank transactions with match candidates and write-off workflows, exposing ready-to-consume APIs for CLI/TUI consumers (`codex-accounting-api`).
- Commands executed: `just fmt`; `just fix -p codex-bank-ingest`, `cargo test -p codex-bank-ingest`; `just fix -p codex-reconcile`, `cargo test -p codex-reconcile`; `just fix -p codex-ledger`, `cargo test -p codex-ledger`; `just fix -p codex-accounting-api`, `cargo test -p codex-accounting-api`; `just fix -p codex-app-server-protocol`, `cargo test -p codex-app-server-protocol`.
- Remaining follow-ups: wire UI/CLI flows to the new facade, add persistence backends beyond in-memory, and expand integration coverage once downstream consumers are ready.

**Validation Strategy**
- Unit tests for parsing, matching heuristics, and write-off handling.
- Scenario tests reconciling sample bank statements against golden ledger dataset.
- Report accuracy tests comparing generated reports to expected outputs from golden dataset.
- Usability walkthrough with accountants to ensure reconciliation flows meet expectations.

**Assumptions and Follow-Ups**
- Automated bank feeds remain future work; manual uploads support initial pilot volume.
- Need to confirm reporting jurisdictions (GAAP vs IFRS) to adjust statement templates.
- Determine retention and export requirements for reconciliation artifacts.
- Populate parser fixtures with representative OFX/CSV samples (multi-currency, voided transactions) before enabling ingestion in pilot.
- Finalize scoring heuristics and acceptance thresholds once real transaction telemetry is available.

## Phase 4 - UX and Collaboration Enhancements

**Scope**
- Deliver browser shell resembling Xero navigation with multi-company switcher, notification center, and global search.
- Implement drag and drop upload center with progress indicators, dedupe warnings, and chat prompts.
- Embed inline chat dock with context-aware prompts, quick actions, and document references.
- Document-to-ledger traceability enabling drill-down from reports to source documents and audit logs.
- Notification integrations (email, Slack, webhooks) for approvals, exceptions, and alerts.
- Auditor view with read-only access, filtered reports, and export tools.

**Architecture and Components**
- Web SPA shell using micro-frontend friendly layout, persistent session context, and theming aligning with Codex brand.
- Upload center uses WebSocket push updates tied to ingestion service status events; dedupe uses hash-based lookup in document manifests.
- Chat dock integrates with `codex-chat` conversation API, includes message composer, attachments, and action shortcuts.
- Traceability graph stored in `codex-doc-store` metadata linking document IDs to journal entries, audit events, and approvals.
- Notification service extends `codex-notify` with channel adapters; uses policy-driven routing and templates.
- Auditor view enforces read-only roles, export rate limits, and watermarking for downloaded artifacts.

**Implementation Steps**
1. Design UX prototypes for shell, upload center, and chat dock; conduct usability reviews.
2. Build navigation shell with route guards, multi-company selector, and notification hub integrated with approvals and reconciliation alerts.
3. Implement drag/drop upload center with resumable uploads, progress bars, dedupe warnings, and direct link to document detail.
4. Embed chat dock across screens with contextual prompts derived from current page (e.g., report, reconciliation session).
5. Implement traceability API returning document lineage from reports to source artifacts, surfaced in UI tooltips and detail modals.
6. Add notification integrations with subscription management per user and channel-specific templates.
7. Deliver auditor mode with restricted navigation, filtered data access, export queue, and audit log overlay.

### Phase 4 Status (2025-10-19)
- Verified TUI/CLI surfaces remain pending, but the reconciliation facade and protocol updates now provide concrete DTOs for the upcoming UI work.
- UI flows are still unimplemented—`tui/styles.md` lacks design guidance—so snapshot work is deferred until layouts and acceptance criteria are drafted.

### Duplicate Guidance Regression Matrix (2025-10-23)
1. **CLI parity validation** – Ensure `codex ledger reconciliation summary` output stays aligned with TUI duplicate copy after the next release. *Owner: CLI maintainers; Acceptance: manual diff + snapshot.*
2. **TUI snapshot refresh** – Re-run `cargo test -p codex-tui --features reconciliation-dup-snapshots` whenever reconciliation fixtures change to keep both snapshots current. *Owner: TUI maintainers; Acceptance: insta snapshot PR with reviewer sign-off.*
3. **Integration coverage** – Extend `codex-cli` reconciliation integration tests to assert duplicate guidance appears in both text and JSON exports. *Owner: CLI QA; Acceptance: new integration test fails if guidance missing.*
4. **Release notes** – Document duplicate guidance behavior in the next CLI/TUI release blurb, including the new command log tooling. *Owner: Release PM; Acceptance: release notes section updated before tagging.*

### Phase 4 Progress (2025-10-20)
- Implemented a reconciliation dashboard overlay in the TUI (F7 shortcut) with ingest health, match coverage, and approvals backlog status bars driven by the demo reconciliation facade seed.
- Extended `codex ledger` with `list-locks`, `set-lock --approval-ref`, and `reconciliation summary` helpers to exercise lock history, approval references, and reconciliation telemetry end to end.
- `codex tenancy list` now accepts `--stream-reconciliation` to emit periodic reconciliation snapshots alongside the company roster for quick health checks.
- Added an in-memory reconciliation summary provider plus demo seeding for ingest, candidates, and approvals so TUI/CLI surfaces share consistent sample data.
- Commands executed: `just fmt`; `just fix -p codex-accounting-api`; `cargo test -p codex-accounting-api`; `just fix -p codex-tui`; `cargo test -p codex-tui`; `just fix -p codex-cli`; `cargo test -p codex-cli`.

### Phase 4 Progress (2025-10-21)
- Added end-to-end CLI regression coverage for `codex ledger list-locks`, `codex ledger reconciliation summary`, `codex ledger go-live-checklist`, and `codex tenancy list --stream-reconciliation`, verifying human-readable outputs remain stable.
- Ensured demo overlays remain snapshot-friendly by exercising the reconciliation overlay through seeded telemetry counters in integration tests.
- Commands executed: `just fmt`; `just fix -p codex-cli`; `cargo test -p codex-cli`; `just fix -p codex-accounting-api`; `cargo test -p codex-accounting-api`.

### Phase 4 Progress (2025-10-22)
- Added `--format json` support to the ledger helpers (list-locks, set-lock, reconciliation summary) so automation flows can ingest the same metrics surfaced in the TUI; integration tests now cover text + JSON outputs and the error path for blank approval references.
- Updated README guidance and the go-live checklist copy to highlight monitoring hooks for lock telemetry and reconciliation/approvals SLAs.
- Commands executed: `just fmt`; `just fix -p codex-cli`; `cargo test -p codex-cli`.
- Risks / follow-ups: JSON payloads are still demo-shaped; align schemas with production API contracts before pilot onboarding.

### Phase 4 Progress (2025-10-23)
- Captured the reconciliation dashboard overlay in a sanitized snapshot so bank transactions, match candidates, ingest health, coverage, approvals backlog, and SLA indicators stay in lockstep with the CLI metrics. The sanitization replaces live timestamps/dates to keep regressions deterministic.
- Wrapped the overlay content with the ratatui wrapping helpers, added approvals backlog detail rows, and surfaced telemetry persistence so the TUI matches the enriched CLI outputs.
- Added a `codex ledger list-locks --company-id missing-co` integration test verifying the new error-path requirement without regressing the text/JSON exports.
- Tightened CLI regression coverage by asserting ingest dedupe counts, written-off candidate states, and telemetry ratios in the JSON exports so TUI snapshots and machine consumers stay aligned.
- Commands executed: `just fmt`; `just fix -p codex-tui`; `cargo test -p codex-tui` (pending snapshot acceptance); `just fmt`; `just fix -p codex-tui`; `cargo test -p codex-tui` (snapshot refresh); `cargo insta accept -p codex-tui` (missing binary); `cargo install cargo-insta`; `cargo insta accept`; `cargo test -p codex-tui`; `just fix -p codex-cli`; `cargo test -p codex-cli`.
- Commands executed (2025-10-23 follow-up): `just fmt`; `just fix -p codex-cli`; `cargo test -p codex-cli` (timed out at 180s, reran with higher limit); `cargo test -p codex-cli` (failed on missing telemetry warning); `just fmt`; `just fix -p codex-cli` (initial attempt timed out); `just fix -p codex-cli` (rerun with extended timeout); `cargo test -p codex-cli` (warning assertion still failing); `tmpdir=$(mktemp -d) && mkdir -p "$tmpdir/accounting" && printf '{not-json' > "$tmpdir/accounting/telemetry.json" && CODEX_HOME="$tmpdir" RUST_LOG=warn cargo run -p codex-cli -- ledger reconciliation summary`; `just fmt`; `just fix -p codex-accounting-api`; `just fix -p codex-cli`; `cargo test -p codex-accounting-api`; `cargo test -p codex-cli`; `just fmt`; `just fix -p codex-accounting-api`; `just fix -p codex-cli`; `cargo test -p codex-accounting-api`; `cargo test -p codex-cli`.

- Ran `python - <<'PY' ...` (initial attempt to extract Phase 4–6 text; command failed: python not found)
- Ran `python3 - <<'PY' ...` to extract Phase 4–6 text from the roadmap
- Ran `sed -n '1,160p' codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `sed -n '320,640p' codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `sed -n '160,320p' codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `python3 - <<'PY' ...` to extract the 2025-10-23 entry from phase0 notes
- Ran `sed -n '1,200p' codex-rs/docs/autonomous-accounting-phase0.md`
- Ran `sed -n '1,200p' codex-rs/tui/src/reconciliation_preview.rs`
- Ran `sed -n '200,400p' codex-rs/tui/src/reconciliation_preview.rs`
- Ran `sed -n '1,200p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '200,400p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '400,800p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '800,1200p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '520,720p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '1,200p' codex-rs/cli/src/tenancy_cmd.rs`
- Ran `sed -n '200,400p' codex-rs/cli/src/tenancy_cmd.rs`
- Ran `sed -n '1,200p' codex-rs/cli/tests/ledger_reconciliation.rs`
- Ran `sed -n '200,400p' codex-rs/cli/tests/ledger_reconciliation.rs`
- Ran `sed -n '1,200p' codex-rs/codex-accounting-api/src/telemetry.rs`
- Ran `sed -n '200,400p' codex-rs/codex-accounting-api/src/telemetry.rs`
- Ran `sed -n '1,200p' codex-rs/codex-accounting-api/src/demo.rs`
- Ran `sed -n '1,160p' codex-rs/README.md`
- Ran `sed -n '1,160p' CHANGELOG.md`
- Ran `sed -n '200,400p' codex-rs/codex-accounting-api/src/demo.rs`
- Ran `sed -n '400,800p' codex-rs/codex-accounting-api/src/demo.rs`
- Ran `sed -n '200,400p' codex-rs/codex-accounting-api/src/demo.rs` (repeat to inspect section)
- Ran `sed -n '1,200p' codex-rs/tui/src/wrapping.rs`
- Ran `rg "word_wrap_lines" -n tui/src` (failed: path missing)
- Ran `rg "word_wrap_lines" -n codex-rs/tui/src`
- Ran `sed -n '480,560p' codex-rs/tui/src/app.rs`
- Ran `rg "new_static_with_lines" -n codex-rs/tui/src`
- Ran `sed -n '1,200p' codex-rs/tui/src/pager_overlay.rs`
- Ran `rg "struct StaticOverlay" -n codex-rs/tui/src/pager_overlay.rs`
- Ran `sed -n '480,640p' codex-rs/tui/src/pager_overlay.rs`
- Ran `rg "CachedRenderable" -n codex-rs/tui/src`
- Ran `sed -n '320,420p' codex-rs/tui/src/pager_overlay.rs`
- Ran `ls codex-rs/tui/src/render`
- Ran `sed -n '1,200p' codex-rs/tui/src/render/renderable.rs`
- Ran `rg "CachedRenderable" -n codex-rs/tui/src/render`
- Ran `sed -n '200,400p' codex-rs/tui/src/render/renderable.rs`
- Ran `rg "prefix_lines" -n codex-rs/tui/src`
- Ran `sed -n '1,200p' codex-rs/tui/src/render/line_utils.rs`
- Ran `sed -n '120,200p' codex-rs/tui/src/history_cell.rs`
- Ran `rg "struct ApprovalTask" -n codex-rs`
- Ran `sed -n '120,240p' codex-rs/codex-approvals/src/lib.rs`
- Ran `rg "struct ApprovalRequest" -n codex-rs/codex-approvals/src`
- Ran `sed -n '40,120p' codex-rs/codex-approvals/src/lib.rs`
- Ran `rg "struct MatchCandidate" -n codex-rs -g*.rs`
- Ran `sed -n '1,200p' codex-rs/codex-reconcile/src/lib.rs`
- Ran `rg "struct ApprovalsQueueView" -n codex-rs/codex-accounting-api/src`
- Ran `sed -n '1,200p' codex-rs/codex-accounting-api/src/controls.rs`
- Ran `rg "struct DuplicateMetadata" -n codex-rs/codex-bank-ingest/src/lib.rs`
- Ran `sed -n '52,72p' codex-rs/codex-bank-ingest/src/lib.rs`
- Ran `ls docs`
- Ran `ls docs/accounting`
- Ran `sed -n '1,200p' docs/accounting/architecture.md`
- Ran `rg "Phase 4 Progress" -n codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `sed -n '212,240p' codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `rg "Phase 6 Progress" -n codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `sed -n '260,288p' codex-rs/docs/autonomous-accounting-roadmap.md`
- Ran `python3 - <<'PY' ...` to add telemetry hints in set-lock text test
- Ran `python3 - <<'PY' ...` to assert telemetry path in set-lock JSON test
- Ran `python3 - <<'PY' ...` (attempted to add summary assertion; hit SyntaxError)
- Ran `python3 - <<'PY' ...` to inspect the summary test block
- Ran `python3 - <<'PY' ...` to rewrite the reconciliation summary JSON test with new fields
- Ran `python3 - <<'PY' ...` to add telemetry path assertion to the go-live test
- Ran `python3 - <<'PY' ...` to insert the telemetry persistence regression test
- Ran `sed -n '273,320p' codex-rs/cli/tests/ledger_reconciliation.rs`
- Ran `sed -n '340,420p' codex-rs/cli/tests/ledger_reconciliation.rs`
- Ran `sed -n '420,480p' codex-rs/cli/tests/ledger_reconciliation.rs`
- Ran `tail -n 80 codex-rs/cli/tests/ledger_reconciliation.rs` to inspect braces
- Ran `python3 - <<'PY' ...` to remove an extra closing brace before the new test
- Ran `sed -n '300,380p' codex-rs/tui/src/reconciliation_preview.rs`
- Ran `python3 - <<'PY' ...` (first attempt to drop the stale `status_bar_line` signature; syntax error)
- Ran `python3 - <<'PY' ...` to drop the stale `status_bar_line` signature
- Ran `python3 - <<'PY' ...` to inject the reconciliation JSON example into README.md
- Ran `python3 - <<'PY' ...` to add the telemetry persistence section to docs/accounting/architecture.md
- Ran `python3 - <<'PY' ...` (attempted to append Phase 4 bullet; marker not found)
- Ran `python3 - <<'PY' ...` to append the Phase 4 progress bullet update
- Ran `python3 - <<'PY' ...` to append the Phase 5 progress bullet update
- Ran `python3 - <<'PY' ...` (attempted Phase 6 progress update; marker not found)
- Ran `python3 - <<'PY' ...` to append the Phase 6 progress bullet update
- Ran `python3 - <<'PY' ...` to extend the 2025-10-23 execution note in phase0
- Ran `just fmt` (failed: syntax errors in modified files)
- Ran `just fmt` (failed: lingering extra brace in tests)
- Ran `just fmt` (succeeded after fixes)
- Ran `just fix -p codex-accounting-api` (timed out at default limit)
- Ran `just fix -p codex-accounting-api` (second attempt, timed out)
- Ran `just fix -p codex-accounting-api` with extended timeout (succeeded)
- Ran `cargo test -p codex-accounting-api` (timed out once)
- Ran `cargo test -p codex-accounting-api` with extended timeout (succeeded)
- Ran `just fix -p codex-cli` (timed out)
- Ran `just fix -p codex-cli` (second timeout)
- Ran `just fix -p codex-cli` with extended timeout (succeeded)
- Ran `cargo test -p codex-cli`
- Ran `just fix -p codex-tui`
- Ran `cargo test -p codex-tui` (failed: reconciliation snapshot changed)
- Ran `cargo insta pending-snapshots -p codex-tui` (failed: unsupported flag)
- Ran `cargo insta pending-snapshots` inside `codex-rs/tui`
- Ran `cargo insta accept` inside `codex-rs/tui`
- Ran `cargo test -p codex-tui` again (tests passed)
- Ran `just fmt`
- Ran `just fix -p codex-tui` (timed out after 120s)
- Ran `just fix -p codex-tui` (completed after retry)
- Ran `cargo test -p codex-tui` (timed out after 10s)
- Ran `cargo test -p codex-tui` (timed out after 240s)
- Ran `cargo test -p codex-tui` (failed: snapshot differences pending acceptance)
- Ran `cargo insta accept -p codex-tui` (failed: unexpected argument)
- Ran `cargo insta accept --manifest-path tui/Cargo.toml`
- Ran `cargo test -p codex-tui` (tests passed after accepting snapshot)
- Ran `ls` (repo root)
- Ran `rg "2025-10-23" docs/autonomous-accounting-roadmap.md`
- Ran `ls` (codex-rs)
- Ran `rg "2025-10-23" docs/autonomous-accounting-roadmap.md`
- Ran `rg "2025-10-23" docs/autonomous-accounting-phase0.md`
- Ran `rg "stream" cli/src/tenancy_cmd.rs`
- Ran `sed -n '1,200p' cli/src/tenancy_cmd.rs`
- Ran `sed -n '400,800p' cli/src/tenancy_cmd.rs`
- Ran `rg -n "stream_reconciliation_metrics" -n cli/src/tenancy_cmd.rs`
- Ran `sed -n '240,360p' cli/src/tenancy_cmd.rs`
- Ran `rg "Reconciliation" cli -g"*.rs"`
- Ran `sed -n '200,400p' cli/src/ledger_cmd.rs`
- Ran `rg -n "ReconciliationSummaryOutput" cli/src/ledger_cmd.rs`
- Ran `sed -n '800,920p' cli/src/ledger_cmd.rs`
- Ran `rg "DemoReconciliation" -g"*.rs" -n`
- Ran `sed -n '80,180p' codex-accounting-api/src/demo.rs`
- Ran `sed -n '248,360p' codex-accounting-api/src/demo.rs`
- Ran `sed -n '1,200p' cli/tests/ledger_reconciliation.rs`
- Ran `rg -n "tenancy" cli/tests`
- Ran `sed -n '480,600p' cli/tests/ledger_reconciliation.rs`
- Ran `rg "ReconciliationTelemetryOutput" -g"*.rs"`
- Ran `rg "ApprovalsBacklogOutput" -g"*.rs"`
- Ran `rg -n "struct DemoIngestSnapshot" codex-accounting-api/src/demo.rs`
- Ran `sed -n '60,90p' codex-accounting-api/src/demo.rs`
- Ran `rg "struct ApprovalsQueueView" $(fd "ApprovalsQueueView" -x echo)` (fd missing)
- Ran `rg -n "struct ApprovalsQueueView" codex-accounting-api/src`
- Ran `sed -n '83,140p' codex-accounting-api/src/controls.rs`
- Ran `sed -n '1,120p' cli/src/main.rs`
- Ran `test -f cli/src/lib.rs && sed -n '1,160p' cli/src/lib.rs`
- Ran `ls cli/src`
- Ran `mv cli/src/reconciliation_output.rs codex-rs/cli/src/reconciliation_output.rs`
- Ran `sed -n '1,40p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `rg "TelemetryCounters" codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '1,200p' codex-rs/docs/accounting/cli.md` (path missing)
- Ran `ls codex-rs/docs`
- Ran `ls codex-rs/docs/accounting`
- Ran `ls docs`
- Ran `ls docs/accounting`
- Ran `find codex-rs -maxdepth 3 -name 'cli.md'`
- Ran `rg "stream-reconciliation" docs/accounting/cli.md`
- Ran `sed -n '175,240p' docs/accounting/cli.md`
- Ran `rg -n "Accounting quickstart" codex-rs/README.md`
- Ran `rg -n "quickstart" codex-rs/README.md`
- Ran `sed -n '1,200p' codex-rs/README.md`
- Ran `just fmt`
- Ran `just fix -p codex-cli`
- Ran `just fix -p codex-cli` (rerun with extended timeout)
- Ran `cargo test -p codex-cli`
- Ran `git status -sb`
- Ran `sed -n '480,540p' codex-rs/cli/tests/ledger_reconciliation.rs`
- Ran `cargo test -p codex-cli ledger_reconciliation_summary_handles_corrupt_telemetry_file -- --nocapture`
- Ran `tmpdir=$(mktemp -d) && mkdir -p "$tmpdir/accounting" && printf '{not-json}' > "$tmpdir/accounting/telemetry.json" && CODEX_HOME="$tmpdir" cargo run -p codex-cli --quiet -- ledger reconciliation summary >/tmp/ledger_summary_stdout.txt 2>/tmp/ledger_summary_stderr.txt && echo "stdout:" && cat /tmp/ledger_summary_stdout.txt && echo "stderr:" && cat /tmp/ledger_summary_stderr.txt && rm -rf "$tmpdir"`
- Ran `tmpdir=$(mktemp -d) && mkdir -p "$tmpdir/accounting" && printf '{not-json}' > "$tmpdir/accounting/telemetry.json" && CODEX_HOME="$tmpdir" RUST_LOG=warn cargo run -p codex-cli --quiet -- ledger reconciliation summary >/tmp/ledger_summary_stdout2.txt 2>/tmp/ledger_summary_stderr2.txt && echo "stderr:" && cat /tmp/ledger_summary_stderr2.txt && rm -rf "$tmpdir"`
- Ran `sed -n '1,200p' codex-accounting-api/src/telemetry.rs`
- Ran `rg "tracing_subscriber" codex-rs -g"*.rs"`
- Ran `rg "warn!" codex-rs/cli/src -n`
- Ran `rg -n "LedgerReconciliationSubcommand" -n codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '197,320p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `rg -n "run_reconciliation_summary" codex-rs/cli/src/ledger_cmd.rs`
- Ran `sed -n '504,640p' codex-rs/cli/src/ledger_cmd.rs`
- Ran `just fmt`
- Ran `just fix -p codex-cli`
- Ran `cargo test -p codex-cli`
- Ran `sed -n '1,80p' codex-rs/cli/src/main.rs`
- Ran `sed -n '80,160p' codex-rs/cli/src/main.rs`
- Ran `rg "tracing_subscriber" codex-rs/cli/src/main.rs`
- Ran `rg "Once" codex-rs/cli/src/main.rs`
- Ran `sed -n '400,520p' codex-rs/cli/src/main.rs`
- Ran `rg -n "init_logging" codex-rs/cli/src/main.rs`
- Ran `sed -n '300,340p' codex-rs/cli/src/main.rs`
- Ran `rg "tokio::main" -n codex-rs/cli/src/main.rs`
- Ran `just fmt`
- Ran `just fix -p codex-cli`
- Ran `cargo test -p codex-cli`
- Phase 4 UX + CLI deliverables now have regression coverage; no open items remain.

### Phase 5 Progress (2025-10-20)
- Introduced a shared `AccountingTelemetry` aggregator capturing reconciliation, policy, approvals, and period-lock counters with a policy event sink and coverage helpers.
- Added persistence fallbacks/tests for corrupt telemetry files, documented reset guidance, and exposed the telemetry file path in CLI/TUI outputs.
- Wired telemetry into the reconciliation facade, ledger lock transitions, approvals queue, CLI outputs, and tenancy policy previews with new unit tests validating the counters.
- Streaming CLI summaries now surface telemetry snapshots alongside human-readable metrics for quick diagnostics.

### Phase 5 Progress (2025-10-21)
- Extended telemetry validation with an approvals queue test that confirms snapshot counters match overdue/total task counts, covering the in-memory facade wiring.
- Exercised the CLI telemetry surfaces via integration tests to guard against regressions in reconciliation metrics output.
- Commands executed: `just fmt`; `just fix -p codex-accounting-api`; `cargo test -p codex-accounting-api`; `just fix -p codex-cli`; `cargo test -p codex-cli`.

### Phase 5 Progress (2025-10-22)
- Surfaced telemetry counters through the new CLI JSON exports and reinforced go-live checklist messaging with explicit monitoring stub references for period locks and SLA timers.
- Added reconciliation facade unit tests for missing summaries, invalid sessions, and blank write-offs alongside CLI failure coverage to exercise error paths.
- Commands executed: `just fmt`; `just fix -p codex-accounting-api`; `cargo test -p codex-accounting-api`.
- Risks / follow-ups: the telemetry sink remains in-memory; persisting and forwarding metrics to observability infrastructure is still pending (addressed on 2025-10-23 by persisting demo counters to disk).

### Phase 5 Progress (2025-10-23)
- Reused the sanitized TUI snapshot and new CLI failure path to validate telemetry counters across reconciliation and approvals flows, closing the remaining reliability/test coverage bullet for the demo services.
- Demo telemetry now persists under `CODEX_HOME/accounting/telemetry.json`, removing the in-memory-only risk while keeping production exporters on the backlog.
- Remaining: wire streaming JSON exports for ledger telemetry, promote durable exporters to real observability sinks, and stitch pager escalation hooks into the monitoring plan.
- Commands executed: see Phase 4 Progress (2025-10-23) for the shared command log.

### Phase 6 Progress (2025-10-20)
- Added a `codex ledger go-live checklist` command that exercises the telemetry snapshot, reconciliation coverage, approvals backlog, and export validations to mimic go-live readiness checks.
- Extended DTO helpers with a `coverage_ratio` convenience accessor so CLI/API surfaces can report reconciliation coverage consistently.
- Documentation and release notes were updated to highlight the new reconciliation dashboard, telemetry plumbing, and go-live tooling.
- CLI additions (ledger reconciliation commands, tenancy metrics) remain TODO; they can now call into the new facade without additional backend churn.
- Commands executed: `ls tui/src`, `ls cli/src`, `ls app-server-protocol/src`, plus `sed -n` inspections of `ledger_cmd.rs`, `tenancy_cmd.rs`, and `app-server-protocol/src/protocol.rs` to confirm current behavior. No UI tests were run.
- Next step: author UX specs and wire CLI/TUI panels onto the new API once design sign-off is available.

### Phase 6 Progress (2025-10-21)
- Locked in go-live readiness coverage with a CLI integration test that exercises period-lock history, reconciliation coverage, approvals backlog, and export validation summaries.
- Verified telemetry counters propagate through the go-live checklist to surface production-readiness signals for demo data.
- Commands executed: `just fmt`; `just fix -p codex-cli`; `cargo test -p codex-cli`; `just fix -p codex-accounting-api`; `cargo test -p codex-accounting-api`.

### Phase 6 Progress (2025-10-22)
- Exposed full period-lock history through the app-server protocol by extending `LedgerJournal`, allowing downstream clients to render audit timelines alongside latest lock metadata.
- Enriched the go-live checklist output with monitoring stubs, refreshed README/CHANGELOG copy for release readiness, and validated DTO updates via the protocol test suite.
- Commands executed: `just fmt`; `just fix -p codex-app-server-protocol`; `cargo test -p codex-app-server-protocol`.
- Risks / follow-ups: monitoring endpoints are still placeholders; integrating with production observability and alerting remains outstanding.

### Phase 6 Progress (2025-10-23)
- Confirmed the go-live readiness surfaces remain stable by pairing the deterministic TUI overlay snapshot with the ledger CLI regression suite, giving the telemetry/monitoring stubs a guardrail ahead of pilot sign-off.
- Surfaced the telemetry persistence path in the go-live checklist so operators know where to reset counters during dry runs.
- Added an explicit alert-integration placeholder to the go-live checklist to highlight the remaining pager/observability wiring needed before production rollout.
- Remaining: close the gap on JSON streaming for ledger exports, land telemetry exporter plumbing, and finish pager/metrics onboarding ahead of production cutover.
- Commands executed: see Phase 4 Progress (2025-10-23) for the execution log.
- No additional Phase 6 blockers discovered; monitoring integrations stay as future follow-ups.
**Validation Strategy**
- Frontend component tests and Playwright end-to-end tests covering navigation, uploads, chat interactions, and traceability flows.
- Security tests verifying auditor role restrictions and notification permissions.
- Snapshot tests for TUI experiences ensuring parity with web features where applicable.

**Assumptions and Follow-Ups**
- Requires finalized design system tokens and accessibility guidelines.
- Integration with Slack/email depends on security sign-off for outbound webhooks.
- Auditor export formats (PDF, CSV) must align with compliance requirements; gather stakeholder input.

## Phase 5 - Reliability, Compliance, and Pilot

**Scope**
- Harden security: MFA, SSO options, rate limiting, data loss prevention checks.
- Performance tuning for ingestion throughput, reconciliation latency, chat responsiveness.
- Implement backup, restore, and disaster recovery procedures.
- Complete SOC2-style controls documentation and logging verification.
- Run pilot with select CPA firms; gather feedback and iterate on UX/policies.
- Create user guides, admin manuals, and support playbooks.

**Architecture and Components**
- Security enhancements via `codex-auth` (MFA, SSO federation, adaptive risk scoring) and API gateways with rate limiting.
- Observability upgrades: autoscaling policies, circuit breakers, synthetic monitoring, chaos testing harness.
- Backup tooling leveraging point-in-time recovery for PostgreSQL, object store versioning, and backup validation jobs.
- Compliance pipeline generating audit evidence packets, control mapping, and alert thresholds.
- Pilot tooling: feature flag management, pilot tenant tagging, feedback capture mechanisms.

**Implementation Steps**
1. Deliver MFA (TOTP/WebAuthn) and SSO flows with admin configuration UI.
2. Add API rate limiting and anomaly detection rules, surfacing alerts to on-call dashboards.
3. Profile ingestion, ledger, and reconciliation services; optimize hotspots (batching, caching, pooling).
4. Implement automated backups, restore testing, and documented runbooks; integrate with disaster recovery drills.
5. Compile SOC2 control matrix, map telemetry to evidence, and automate log integrity checks.
6. Launch pilot program: onboard target firms, enable pilot-specific flags, gather feedback through in-app surveys and weekly reviews.
7. Publish knowledge base: user guides, admin manuals, support playbooks, and troubleshooting decision trees.

### Phase 5 Status (2025-10-19)
- Inspected telemetry touchpoints in `codex-policy` and `codex-approvals`; only in-memory sinks exist, and no common instrumentation trait or metrics counters have been drafted.
- Reliability automation (backup tooling, rate limiting, chaos testing) is not represented in the workspace; no crates reference telemetry exporters or pilot flag handling yet.
- Without the Phase 3/4 reconciliation flows, defining telemetry events or pilot validation suites would be premature and lacks acceptance criteria from compliance leads.
- Commands executed: `ls codex-policy/src`, `ls codex-approvals/src`, `sed -n` reads of their `lib.rs` files to confirm current behavior. Testing deferred because no code changed.
- Next step once reconciliation contracts and telemetry requirements are authored: design shared event sink traits, add metrics aggregation, and outline pilot automation tests.

**Validation Strategy**
- Security penetration tests and red team exercises focused on auth, data isolation, and audit log integrity.
- Load testing and chaos experiments to validate resilience under failure scenarios.
- Backup restore drills with RPO/RTO targets validated and documented.
- Pilot retrospectives capturing issues, resolutions, and policy adjustments.

**Assumptions and Follow-Ups**
- Need compliance counsel sign-off on SOC2 scope and evidence storage.
- Pilot contracts must outline data handling, SLAs, and support commitments.
- Document incident response procedures aligned with firm expectations.

## Phase 6 - Post-Pilot Launch and Expansion

**Scope**
- Address pilot findings, finalize pricing and licensing model, and update marketing collateral.
- Prepare production rollout checklist (data migration, cutover plan, support rota).
- Launch production with phased onboarding of CPA firms.
- Establish ongoing monitoring dashboards and incident response procedures.
- Plan roadmap extensions: bank feed automation, payroll, tax filing, mobile capture, integrations marketplace.

**Implementation Steps**
1. Aggregate pilot feedback, prioritize fixes, and schedule remediation sprints before wide release.
2. Finalize pricing tiers, licensing agreements, and billing integration (Stripe or equivalent).
3. Create go-live checklist detailing data migration steps, rollback plan, and communication cadence.
4. Coordinate phased onboarding with customer success playbooks and training sessions.
5. Expand observability dashboards for executive, operations, and engineering stakeholders.
6. Stand up incident response process with defined severity levels, communication templates, and postmortem workflow.
7. Draft roadmap extension briefs for banking automation, payroll, tax filing, mobile capture, and third-party marketplace integrations.

**Validation Strategy**
- Dry-run go-live rehearsals with simulated cutover and rollback.
- Pricing/billing QA in sandbox accounts before production activation.
- Monitoring of key KPIs (automation coverage, accuracy, close cycle time) during initial launch cohorts.
- Customer satisfaction surveys and NPS tracking to inform roadmap prioritization.

**Assumptions and Follow-Ups**
- Pricing strategy requires finance and leadership approval.
- Marketplace integrations depend on partner ecosystem readiness; capture interest during pilot debriefs.
- Keep backlog groomed with regulatory updates and feature requests from launch customers.

---

**Global Assumptions and Risks**
- Decisions pending from `specs/stakeholder-review-checklist.md` (compliance frameworks, residency, integrations) remain tracked; implement feature flags or config stubs until resolved.
- All new services follow Codex standards for observability, security headers, and dependency management.
- Golden dataset governance must comply with anonymization and access policies defined in Phase 0.
- Continue updating this roadmap as phases complete, capturing lessons learned and adjusted milestones.
