# Accounting CLI Reference

This guide summarizes the accounting-focused Codex CLI commands and the structured payloads they emit. All examples below use demo data seeded by the CLI; replace placeholder values (such as `<CODEX_HOME>`) with paths from your environment.

## `codex ledger reconciliation summary`

### Text mode
- Prints matched vs. pending counts, coverage ratios, ingest dedupe metrics, approvals backlog, and the persisted telemetry file path.

Example excerpt:

```text
Reconciliation summary for Demo Manufacturing
- Coverage: 8 matched / 3 pending (73%)
- Ingest: 5/6 deduped • last feed 2025-10-23T12:00:00Z
- Approvals backlog: 1 overdue / 3 open (generated 2025-10-23T11:59:00Z)
- Telemetry counters: tx 5 • candidates 5 • write-offs 0
Telemetry file: <CODEX_HOME>/accounting/telemetry.json
```

### JSON mode
- Use `--format json` to retrieve machine-readable output.
- `telemetry_path` points to the persisted counters (available when `CODEX_HOME` is set).
- Duplicate metadata and write-off details surface in `transactions` and `candidates`.

```json
{
  "company_id": "demo-co",
  "company_name": "Demo Manufacturing",
  "matched": 8,
  "pending": 3,
  "coverage_ratio": 0.73,
  "ingest": {
    "ingested_total": 6,
    "deduped_total": 5,
    "duplicates_dropped": 1,
    "last_feed_at": "2025-10-23T12:00:00Z"
  },
  "approvals": {
    "generated_at": "2025-10-23T11:59:00Z",
    "total": 3,
    "overdue": 1
  },
  "telemetry_path": "<CODEX_HOME>/accounting/telemetry.json",
  "transactions": [
    {
      "transaction_id": "txn-001",
      "posted_date": "2025-10-22",
      "description": "Stripe payout",
      "amount_minor": 12500,
      "currency": "USD",
      "account_id": "operating-cash",
      "source_reference": "REF-001",
      "source_checksum": null,
      "is_void": false,
      "duplicates_dropped": 1,
      "duplicate_group": "grp-001"
    }
  ],
  "candidates": [
    {
      "transaction_id": "txn-002",
      "journal_entry_id": "je-demo-1",
      "status": "WrittenOff",
      "score": 0.82,
      "group_id": "grp-utility",
      "write_off_reference": "APR-UTILITY-ADJ"
    }
  ],
  "telemetry": {
    "reconciliation_transactions": 5,
    "reconciliation_candidates": 5,
    "reconciliation_write_offs": 0,
    "period_lock_events": 3,
    "period_lock_soft_close": 2,
    "period_lock_close": 1,
    "period_lock_reopen_soft": 0,
    "period_lock_reopen_full": 0,
    "policy_auto_post": 0,
    "policy_needs_approval": 0,
    "policy_reject": 0,
    "approvals_total": 3,
    "approvals_overdue": 1
  }
}
```

## `codex ledger list-locks`

### Text mode
- Lists the lock timeline for the demo general ledger, including approval references and telemetry hints.

Example excerpt:

```text
Lock history for Demo Manufacturing (jnl-gl)
- 2024/01 SoftClose (approval APR-LCK-001) by demo@example.com
- 2024/01 Close (approval APR-LCK-002) by demo@example.com
- 2024/02 SoftClose (approval APR-LCK-003) by demo@example.com
Telemetry: events 3 • soft-close 2 • close 1 • reopen soft 0 • reopen full 0
Telemetry file: <CODEX_HOME>/accounting/telemetry.json
```

### JSON mode

```json
{
  "company_id": "demo-co",
  "company_name": "Demo Manufacturing",
  "journal_id": "jnl-gl",
  "locks": [
    {
      "fiscal_year": 2024,
      "period": 1,
      "action": "SoftClose",
      "approval_reference": "APR-LCK-001",
      "locked_at": "2025-10-23T12:00:00Z",
      "locked_by": "demo@example.com"
    },
    {
      "fiscal_year": 2024,
      "period": 1,
      "action": "Close",
      "approval_reference": "APR-LCK-002",
      "locked_at": "2025-10-23T12:10:00Z",
      "locked_by": "demo@example.com"
    },
    {
      "fiscal_year": 2024,
      "period": 2,
      "action": "SoftClose",
      "approval_reference": "APR-LCK-003",
      "locked_at": "2025-10-23T12:20:00Z",
      "locked_by": "demo@example.com"
    }
  ],
  "telemetry": {
    "events": 3,
    "soft_close": 2,
    "close": 1,
    "reopen_soft": 0,
    "reopen_full": 0
  },
  "telemetry_path": "<CODEX_HOME>/accounting/telemetry.json"
}
```

## `codex ledger set-lock`

- `--format json` returns the updated lock snapshot and telemetry increment.
- Text mode echoes the lock change and the same telemetry file hint.

```json
{
  "journal_id": "jnl-gl",
  "period": {
    "fiscal_year": 2024,
    "period": 5
  },
  "action": "Close",
  "approval_reference": "CLI-JSON-APR",
  "locked_at": "2025-10-23T12:30:00Z",
  "locked_by": "demo@example.com",
  "telemetry": {
    "events": 4,
    "soft_close": 2,
    "close": 2,
    "reopen_soft": 0,
    "reopen_full": 0
  },
  "telemetry_path": "<CODEX_HOME>/accounting/telemetry.json"
}
```

## `codex tenancy list --stream-reconciliation`

- When companies exist, prints the roster and optional stream.
- Without companies (the default demo firm), the command streams three reconciliation ticks to showcase live metrics.
- Each tick includes coverage, ingest dedupe stats, and telemetry counters. After the third tick, a telemetry file path is printed if counters are persisted.
- Pass `--json` alongside `--stream-reconciliation` to emit newline-delimited JSON snapshots (two ticks) that include coverage metrics, approvals backlog totals, ingest dedupe stats, telemetry counters, and the resolved telemetry path.

Example flow:

```text
No companies found for firm demo-firm.
Reconciliation metrics (demo feed):
  tick 1: matched 8 | pending 3 | coverage 73% | backlog 1 overdue / 3 open
            ingest 5/6 deduped • last feed 2025-10-23T12:00:00Z
            telemetry tx 5 • candidates 5 • write-offs 0
  tick 2: matched 8 | pending 3 | coverage 73% | backlog 1 overdue / 3 open
            ingest 5/6 deduped • last feed 2025-10-23T12:00:00Z
            telemetry tx 6 • candidates 6 • write-offs 0
  tick 3: matched 8 | pending 3 | coverage 73% | backlog 1 overdue / 3 open
            ingest 5/6 deduped • last feed 2025-10-23T12:00:00Z
            telemetry tx 7 • candidates 7 • write-offs 0
Telemetry file: <CODEX_HOME>/accounting/telemetry.json
```

### JSON streaming (`--json`)

Structured streaming returns newline-delimited JSON payloads for automation. Example (sanitized):

```json
{"tick":1,"matched":8,"pending":3,"coverage_ratio":0.727,"coverage_percent":72.7,"approvals":{"generated_at":"2025-10-23T12:00:00Z","total":3,"overdue":1},"ingest":{"ingested_total":6,"deduped_total":5,"duplicates_dropped":1,"last_feed_at":"2025-10-23T09:00:00Z"},"telemetry":{"reconciliation_transactions":7,"reconciliation_candidates":6,"reconciliation_write_offs":0,"period_lock_events":0,"period_lock_soft_close":0,"period_lock_close":0,"period_lock_reopen_soft":0,"period_lock_reopen_full":0,"policy_auto_post":0,"policy_needs_approval":0,"policy_reject":0,"approvals_total":3,"approvals_overdue":1},"telemetry_path":"/tmp/codex-accounting/telemetry.json","generated_at":"2025-10-23T12:00:00Z"}
{"tick":2,"matched":8,"pending":2,"coverage_ratio":0.800,"coverage_percent":80.0,"approvals":{"generated_at":"2025-10-23T12:00:00Z","total":2,"overdue":0},"ingest":{"ingested_total":6,"deduped_total":5,"duplicates_dropped":1,"last_feed_at":"2025-10-23T09:00:00Z"},"telemetry":{"reconciliation_transactions":8,"reconciliation_candidates":7,"reconciliation_write_offs":0,"period_lock_events":0,"period_lock_soft_close":0,"period_lock_close":0,"period_lock_reopen_soft":0,"period_lock_reopen_full":0,"policy_auto_post":0,"policy_needs_approval":0,"policy_reject":0,"approvals_total":2,"approvals_overdue":0},"telemetry_path":"/tmp/codex-accounting/telemetry.json","generated_at":"2025-10-23T12:00:00Z"}
```

## Resetting telemetry counters

Telemetry persists under `<CODEX_HOME>/accounting/telemetry.json`. To reset counters for a fresh demo run:

1. Delete the file (or remove the `<CODEX_HOME>/accounting` directory).
2. Re-run any ledger or tenancy command; Codex will recreate the file and log a warning if the previous contents were malformed.

Refer to the go-live checklist output for reminders to clear telemetry prior to production runs.
