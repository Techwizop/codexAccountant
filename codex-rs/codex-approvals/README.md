# codex-approvals

Approval queue service scaffolding for Codex autonomous accounting:

- Models approval requests with SLA metadata, priority, and assignment tracking.
- Exposes an async service trait plus an in-memory implementation used by CLI/testing workflows.
- Supports optimistic assignment, multi-stage sequential approvals, SLA breach detection, and filtered queue listings for integration with UI/TUI surfaces.
- Provides a queue export snapshot for audit-log ingestion and reporting.

## TODO
- Persist approval state to durable storage (PostgreSQL/Redis) with outbox for notifications.
- Add concurrency-safe selectors for high-volume assignment (compare-and-swap or optimistic locking).
- Wire notifications and audit log hooks for decision capture and SLA breach alerts.
