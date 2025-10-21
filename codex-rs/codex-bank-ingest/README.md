# codex-bank-ingest

Bank statement ingestion primitives for Codex autonomous accounting.

- Defines normalized transaction model shared across parsers.
- Exposes parser traits for OFX and CSV feeds (stubs for now).
- Provides dedupe helpers to collapse provider retries and duplicate exports.

## Pipeline Outline
- Fetch/upload statement payload (CSV, OFX, PDF-to-CSV) and store raw artifact.
- Parse into `NormalizedBankTransaction` records with provider metadata and checksums.
- Run dedupe + ordering to guard downstream reconciliation.
- Emit stream/batch into reconciliation service and ledger suggestion engine.

## Configuration Profiles

CSV imports rely on a profile describing how to map provider headers into the normalized transaction schema. Profiles are authored as JSON (see `tests/fixtures/csv/profile.json`) and support:

- Column mapping for required and optional fields (transaction id, account id, amount, currency, description, source reference, checksum, void flag).
- Date format selection via `date_format` (defaults to `%Y-%m-%d`).
- Decimal handling via `amount_minor_factor` (defaults to `100` for cents).

The streaming parser enforces ISO-4217 currency codes, computes missing checksums from key fields, and captures duplicate metadata that is consumed by the dedupe helper.

## Dedupe Metrics

`dedupe_transactions` returns a `DedupeOutcome` containing the canonical transaction list alongside metrics (`kept`, `dropped`). Each kept transaction tracks its duplicate group, the number of occurrences observed, and the identifiers that were discarded.

## Fixtures

Sample CSV and OFX statements live under `tests/fixtures/` and are exercised by unit tests covering multi-currency handling, voided entries, duplicates, and end-to-end parser behavior.
