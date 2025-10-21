# codex-reconcile

Reconciliation service scaffolding for Codex autonomous accounting.

- Defines match proposal scoring with weighted heuristics (amount delta, posting delta, description similarity).
- Provides session lifecycle management including partial accept groups, write-offs, full acceptance, and reopen flows.
- Ships an audit hook surface and trait-based persistence abstraction with an in-memory store; a feature-gated Postgres stub documents the planned durable backend.

## Integration Notes

- Consumers should register a `ReconciliationAuditHook` to forward important lifecycle events into their logging or notification systems.
- Use the `ReconciliationStore` trait to supply custom persistence. The `postgres-store` feature exposes a stub implementation wired for future async SQL integration.
- Match proposals should normalize amounts into minor units and provide descriptive text for best scoring results; the scoring strategy expects human-readable phrases.
- Session reopen resets candidate statuses to `Pending`; downstream services must re-evaluate acceptance state before posting ledger entries.
