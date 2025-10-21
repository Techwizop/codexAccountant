# codex-audit-log

Append-only ledger for audit events with:

- Hash-chain envelope to detect tampering across contiguous records.
- Trait-based append/stream interface for plugging alternative storage backends.
- In-memory implementation used by tests and demos.
- Test coverage validating append semantics and tamper detection.
