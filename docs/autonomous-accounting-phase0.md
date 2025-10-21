# Autonomous Accounting - Phase 0 Foundations

This document captures the foundational decisions and assumptions prepared during Phase 0 of the autonomous accounting roadmap. It aligns with `specs/autonomous-accounting-spec.md`, `specs/tasks.md`, and open stakeholder items in `specs/stakeholder-review-checklist.md`.

## 0.1 Compliance & Legal Baseline
- **Confirmed baselines:** Start with US GAAP controls and record retention, require immutable audit trails, and plan SOC2-aligned logging. IFRS and regional variants remain in scope; treat them as configuration-driven profiles, not separate builds.
- **Retention guardrails:** Mirror CPA guidance with a seven-year minimum retention for financial statements, supporting documents, and audit trails; allow tenant-level overrides where regulation demands longer storage.
- **Approval thresholds:** Default autonomous postings to require human review above USD 5,000 or whenever AI confidence drops below a configurable floor. Always escalate payroll, owner distributions, and tax filings for review regardless of amount.
- **Safest assumptions (flagged):** Introduce a `compliance.default_framework` config stub (no runtime change yet) to select GAAP versus IFRS once stakeholders decide. Plan to extend policies for GDPR and data residency controls before onboarding EU clients.
- **Outstanding decisions:** Confirm jurisdictions for the first pilot, mandated certifications (SOC2 Type II, ISO 27001, others), and cross-border data residency rules. Track these items in the stakeholder log for follow-up.

## 0.2 Hosting, Tenancy, and Security Baseline
- **Hosting model:** Multi-tenant SaaS with per-firm logical isolation plus an optional dedicated environment (VPC-isolated) for regulated clients. Deploy on managed cloud (AWS baseline) with region pinning to satisfy residency mandates.
- **Access controls:** Role-based permissions mapped to CPA hierarchy (Partner, Senior, Staff, Auditor). Require MFA for elevated roles; add SSO via SAML or OIDC as the roadmap progresses.
- **Encryption posture:** Enforce TLS 1.3 for data in transit, use envelope-encrypted object storage for documents, and manage regional KMS keys per tenant. Keep ledger and policy databases encrypted at rest with automated key rotation.
- **Audit logging:** Centralized append-only log service with tamper detection, streamed to a secure archive for the full retention window.
- **Assumptions:** Preserve existing sandbox behavior for agent-initiated processes and avoid touching `CODEX_SANDBOX_*` handling. Defer regional failover until the Phase 5 hardening track.

## 0.3 Target Tech Stack
- **Frontend:** React and TypeScript single-page app with a design system aligned to the Codex CLI look and feel. Continue the Ratatui-based TUI for CLI users and expose a shared GraphQL and REST layer via `codex-accounting-api`.
- **Services:** Rust microservices for the ledger (`codex-ledger`), document intake (`codex-docs` placeholder), policy and approvals (`codex-policy`), notifications, and chat agent orchestration leveraging existing Codex core crates.
- **Data stores:** PostgreSQL for ledger, policy, and authentication data; S3-compatible object storage for documents; managed Redis for queues and caching; and an analytics warehouse (BigQuery or Snowflake) fed through change data capture.
- **AI and ML layer:** Codex agent harness with task-specific prompts, pluggable LLM endpoints, retrieval across document embeddings, and optional fine-tuned models stored under firm-specific namespaces.
- **Integrations:** REST and webhook gateway for external systems. Banking feed adapters remain a Phase 3 deliverable, but define the interfaces now.

## 0.4 Environments and CICD
- **Environment tiers:** Local development, shared integration, staging (mirrors production services with anonymized data), pilot (limited client firms), and production. Each stage enforces infrastructure-as-code parity.
- **Deployment pipeline:** Git-based workflow with mandatory code review, automated lint and test gates, image builds, and progressive delivery. Use blue/green deployments for stateless services and guarded migrations for stateful components.
- **Observability:** Centralized tracing through OpenTelemetry, metrics dashboards, and structured logs per environment. Align alert policies with on-call rotations before the pilot kickoff.
- **Release cadence:** Bi-weekly release train during the MVP phase, promoting to pilot after staging soak tests. Maintain an emergency hotfix lane with an approval workflow.

## 0.5 Golden Dataset Strategy
- **Dataset makeup:** Curate anonymized ledgers, invoices, receipts, and bank statements for at least three archetypal clients (retail, professional services, nonprofit). Include recurring transactions, multi-currency samples, and edge cases flagged in the product spec.
- **Anonymization:** Strip PII and replace it with deterministic tokens; keep the mapping in an encrypted vault for troubleshooting under strict controls.
- **Storage and access:** Store datasets in a segregated object storage bucket with read-only IAM roles. Gate access through an approval workflow and audit every read.
- **Validation usage:** Use the dataset for regression tests across ingestion, extraction, posting accuracy, and reconciliation. Wire the suite into CI to guard automation metrics and track accuracy deltas from release to release.
- **Open tasks:** Secure stakeholder approval on source firms, finalize data sharing agreements, and define a refresh cadence (at least quarterly).

## 2025-10-23 Execution Notes
- Persisted demo telemetry counters to `CODEX_HOME/accounting/telemetry.json`, keeping the sandbox-ready CLI/TUI flows aligned with go-live monitoring expectations.
- Commands executed: `just fmt`; `just fix -p codex-accounting-api` (rerun after timeout); `just fix -p codex-cli`; `cargo test -p codex-accounting-api`; `cargo test -p codex-cli`.
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


## Next Steps
- Schedule the stakeholder review to resolve open compliance and residency decisions.
- Draft architecture RFCs for ledger, policy, and document services using the baselines above.
- Begin environment bootstrap scripts (Terraform or equivalent) aligned with the hosting and security assumptions.
