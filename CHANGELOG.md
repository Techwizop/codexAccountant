## Unreleased

- Added `--format json` support to the demo ledger helpers (`codex ledger list-locks`, `codex ledger set-lock`, and `codex ledger reconciliation summary`) and expanded integration tests to cover machine output plus approval-reference error paths.
- Surfaced full period-lock history through the app-server protocol and refreshed the go-live checklist copy with telemetry/monitoring stubs for reconciliation SLAs.
- Hardened reconciliation telemetry error handling via new facade tests and updated README/roadmap notes for the Phase 4-6 accounting workflows.
- Persisted demo telemetry counters to `CODEX_HOME/accounting/telemetry.json`, tightened CLI JSON parity checks, and added an alert-integration placeholder to the go-live checklist.
- Initialized CLI logging for ledger commands so corrupted telemetry files emit visible warnings, added a regression test covering the corrupt JSON path, and expanded the go-live checklist plus docs/roadmap entries with export validation pointers, monitoring TODOs, and telemetry reset guidance.

- Wrapped the reconciliation TUI overlay (approvals backlog details, dedupe metadata) and extended CLI summaries with `telemetry_path`, duplicate indicators, and write-off references; documented telemetry reset/persistence and added regression tests for corrupt files and counter persistence.

The changelog can be found on the [releases page](https://github.com/openai/codex/releases)
