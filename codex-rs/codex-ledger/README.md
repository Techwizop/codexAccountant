# codex-ledger

Foundational accounting domain models and service contracts for the Codex multi-company ledger.

This crate defines:

- Core data structures such as `Company`, `Account`, `Journal`, and `JournalEntry` aligned with the
  design in `docs/accounting/architecture.md`.
- Enumerations and helper types for currencies, tax configuration, and tenant/RBAC context.
- The `LedgerService` trait describing the high-level operations (company creation, account
  management, posting, period control, FX revaluation, and audit queries) that downstream
  implementations will satisfy.

The crate currently contains type definitions, invariants, and unit tests only. Persistence,
integration, and protocol wiring will be added in future milestones.
