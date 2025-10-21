# codex-policy

Policy evaluation engine for Codex autonomous accounting:

- Defines configurable rule sets for auto-post thresholds, flagged vendors/accounts, and AI confidence gating.
- Provides an async trait-based store contract with in-memory and durable adapters; a Postgres-backed persistence stub ships behind the `postgres-store` feature flag.
- Exposes a lightweight evaluation engine returning structured triggers that feed approval flows, and emits telemetry events via pluggable sinks.

## Postgres schema (draft)

```sql
CREATE TABLE policy_rule_sets (
    company_id TEXT PRIMARY KEY,
    rules JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## TODO
- Implement the Postgres adapter (connection management, migrations, optimistic locking).
- Stream `PolicyEvaluationEvent` records to the Codex analytics/event bus once that pipeline lands.
