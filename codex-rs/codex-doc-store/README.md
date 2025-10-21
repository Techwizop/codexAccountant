# codex-doc-store

Codex document storage façade providing:

- S3-compatible object API with logical tenant segregation.
- Envelope-encryption hook so providers can wrap per-object keys.
- Metadata indexing schema covering firm/company scope, tags, retention class, and versions.
- Retention scheduler trait to integrate purge/hold lifecycles.
- In-memory placeholder implementation plus unit tests verifying isolation and retention hooks.
