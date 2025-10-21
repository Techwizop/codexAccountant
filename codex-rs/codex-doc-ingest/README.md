# codex-doc-ingest

Phase 1 ingestion façade providing:

- Axum router skeleton for upload URL and status endpoints.
- Trait-based queue producer and upload signer abstractions.
- Shared DTOs representing upload requests, signed responses, and ingestion events.
- In-memory mock service + CLI harness helper for simulating signed upload URLs during development.
