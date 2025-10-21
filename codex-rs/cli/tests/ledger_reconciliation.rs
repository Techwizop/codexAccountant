use std::fs;
use std::path::Path;

use anyhow::Result;
use assert_cmd::Command;
use pretty_assertions::assert_eq;
use serde_json::Value;
use tempfile::TempDir;

fn codex_command(home: &Path) -> Result<Command> {
    let mut cmd = Command::cargo_bin("codex")?;
    cmd.env("CODEX_HOME", home);
    Ok(cmd)
}

#[test]
fn ledger_list_locks_reports_history() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd.args(["ledger", "list-locks"]).output()?;
    assert!(
        output.status.success(),
        "list-locks exit status: {:?}",
        output.status.code()
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Lock history for Demo Manufacturing"),
        "stdout missing lock history: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry:"),
        "stdout missing telemetry snapshot: {stdout}"
    );
    assert!(
        stdout.contains("approval"),
        "stdout missing approval reference detail: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry file:"),
        "stdout missing telemetry file hint: {stdout}"
    );
    Ok(())
}

#[test]
fn ledger_list_locks_json_emits_structured_data() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args(["ledger", "list-locks", "--format", "json"])
        .output()?;
    assert!(
        output.status.success(),
        "list-locks json exit status: {:?}",
        output.status.code()
    );
    let stdout = String::from_utf8(output.stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert!(
        value
            .get("company_name")
            .and_then(Value::as_str)
            .is_some_and(|name| name.contains("Demo")),
        "json missing company name: {value}"
    );
    assert!(
        value
            .get("locks")
            .and_then(Value::as_array)
            .is_some_and(|locks| !locks.is_empty()),
        "json missing locks: {value}"
    );
    let locks = value
        .get("locks")
        .and_then(Value::as_array)
        .expect("lock array");
    assert_eq!(locks.len(), 3, "expected three demo lock events: {locks:?}");
    let first_lock = locks
        .first()
        .and_then(Value::as_object)
        .expect("lock object");
    assert!(
        first_lock
            .get("approval_reference")
            .is_some_and(|approval| !approval.is_null()),
        "lock entries must surface approval references: {locks:?}"
    );
    assert_eq!(
        first_lock
            .get("action")
            .and_then(Value::as_str)
            .expect("action string"),
        "SoftClose"
    );
    let telemetry = value
        .get("telemetry")
        .and_then(Value::as_object)
        .expect("telemetry object");
    assert!(
        telemetry
            .get("events")
            .and_then(Value::as_u64)
            .is_some_and(|events| events >= 3),
        "telemetry events too low: {value}"
    );
    assert!(
        value
            .get("telemetry_path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.ends_with("telemetry.json")),
        "json missing telemetry path: {value}"
    );
    Ok(())
}

#[test]
fn ledger_list_locks_errors_for_unknown_company() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args(["ledger", "list-locks", "--company-id", "missing-co"])
        .output()?;
    assert!(
        !output.status.success(),
        "list-locks should fail when company missing"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("Company missing-co not found"),
        "stderr missing company error: {stderr}"
    );
    Ok(())
}

#[test]
fn ledger_set_lock_requires_approval_reference() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args([
            "ledger",
            "set-lock",
            "--journal-id",
            "jnl-gl",
            "--fiscal-year",
            "2024",
            "--period",
            "4",
            "--action",
            "close",
            "--approval-ref",
            "CLI-TEST-APR",
        ])
        .output()?;
    assert!(output.status.success(), "set-lock should succeed");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Updated journal jnl-gl period 2024/4 to Close"),
        "stdout missing lock update: {stdout}"
    );
    assert!(
        stdout.contains("approval CLI-TEST-APR"),
        "stdout missing approval reference: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry:"),
        "stdout missing telemetry counters: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry file:"),
        "stdout missing telemetry path hint: {stdout}"
    );
    Ok(())
}

#[test]
fn ledger_set_lock_json_returns_latest_lock() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args([
            "ledger",
            "set-lock",
            "--journal-id",
            "jnl-gl",
            "--fiscal-year",
            "2024",
            "--period",
            "5",
            "--action",
            "close",
            "--approval-ref",
            "CLI-JSON-APR",
            "--format",
            "json",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "set-lock json should succeed: {:?}",
        output.status.code()
    );
    let stdout = String::from_utf8(output.stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(
        value
            .get("action")
            .and_then(Value::as_str)
            .expect("action string"),
        "Close"
    );
    assert_eq!(
        value
            .get("approval_reference")
            .and_then(Value::as_str)
            .expect("approval string"),
        "CLI-JSON-APR"
    );
    let telemetry = value
        .get("telemetry")
        .and_then(Value::as_object)
        .expect("telemetry object");
    assert!(
        telemetry
            .get("events")
            .and_then(Value::as_u64)
            .is_some_and(|events| events >= 1),
        "telemetry missing events: {value}"
    );
    assert!(
        value
            .get("telemetry_path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.ends_with("telemetry.json")),
        "json missing telemetry path: {value}"
    );
    Ok(())
}

#[test]
fn ledger_set_lock_fails_without_meaningful_approval_ref() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args([
            "ledger",
            "set-lock",
            "--journal-id",
            "jnl-gl",
            "--fiscal-year",
            "2024",
            "--period",
            "6",
            "--action",
            "close",
            "--approval-ref",
            "",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "set-lock without approval should fail"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("approval reference must be provided"),
        "stderr missing failure message: {stderr}"
    );
    Ok(())
}

#[test]
fn ledger_reconciliation_summary_highlights_metrics() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    cmd.env("RUST_LOG", "warn");
    let output = cmd.args(["ledger", "reconciliation", "summary"]).output()?;
    assert!(
        output.status.success(),
        "reconciliation summary should succeed"
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Reconciliation summary for Demo Manufacturing"),
        "stdout missing summary header: {stdout}"
    );
    assert!(
        stdout.contains("Coverage"),
        "stdout missing coverage line: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry counters"),
        "stdout missing telemetry counters: {stdout}"
    );
    assert!(
        stdout.contains("write-offs"),
        "stdout missing write-off telemetry detail: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry file:"),
        "stdout missing telemetry file note: {stdout}"
    );
    Ok(())
}

#[test]
fn ledger_reconciliation_summary_json_exposes_metrics() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args(["ledger", "reconciliation", "summary", "--format", "json"])
        .output()?;
    assert!(
        output.status.success(),
        "reconciliation json should succeed"
    );
    let stdout = String::from_utf8(output.stdout)?;
    let value: Value = serde_json::from_str(&stdout)?;
    assert_eq!(
        value
            .get("matched")
            .and_then(Value::as_u64)
            .expect("matched count") as usize,
        8
    );
    assert_eq!(
        value
            .get("pending")
            .and_then(Value::as_u64)
            .expect("pending count") as usize,
        3
    );
    assert!(
        value
            .get("coverage_ratio")
            .and_then(Value::as_f64)
            .is_some_and(|ratio| ratio > 0.0),
        "json missing coverage ratio: {value}"
    );
    let ingest = value
        .get("ingest")
        .and_then(Value::as_object)
        .expect("ingest object");
    assert_eq!(
        ingest
            .get("ingested_total")
            .and_then(Value::as_u64)
            .expect("ingested total"),
        6
    );
    assert_eq!(
        ingest
            .get("deduped_total")
            .and_then(Value::as_u64)
            .expect("deduped total"),
        5
    );
    assert_eq!(
        ingest
            .get("duplicates_dropped")
            .and_then(Value::as_u64)
            .expect("duplicates dropped"),
        1
    );
    let telemetry_path = value
        .get("telemetry_path")
        .and_then(Value::as_str)
        .expect("telemetry path");
    assert!(
        telemetry_path.ends_with("telemetry.json"),
        "telemetry path should end with telemetry.json: {telemetry_path}"
    );
    let telemetry = value
        .get("telemetry")
        .and_then(Value::as_object)
        .expect("telemetry object");
    assert_eq!(
        telemetry
            .get("reconciliation_transactions")
            .and_then(Value::as_u64)
            .expect("transactions telemetry"),
        5
    );
    assert_eq!(
        telemetry
            .get("reconciliation_candidates")
            .and_then(Value::as_u64)
            .expect("candidate telemetry"),
        5
    );
    assert_eq!(
        telemetry
            .get("reconciliation_write_offs")
            .and_then(Value::as_u64)
            .expect("write-off telemetry"),
        0
    );
    let transactions = value
        .get("transactions")
        .and_then(Value::as_array)
        .expect("transactions array");
    assert!(
        !transactions.is_empty(),
        "json missing transactions: {value}"
    );
    assert!(
        transactions.iter().filter_map(Value::as_object).any(|tx| tx
            .get("duplicates_dropped")
            .and_then(Value::as_u64)
            .is_some_and(|dup| dup >= 1)),
        "transactions missing duplicate metadata: {transactions:?}"
    );
    let first_tx = transactions
        .iter()
        .filter_map(Value::as_object)
        .next()
        .expect("first transaction");
    assert!(
        first_tx.contains_key("account_id"),
        "transaction missing account_id: {first_tx:?}"
    );
    assert!(
        first_tx.contains_key("source_reference"),
        "transaction missing source reference: {first_tx:?}"
    );
    let candidates = value
        .get("candidates")
        .and_then(Value::as_array)
        .expect("candidate array");
    let candidate_statuses: Vec<_> = candidates
        .iter()
        .filter_map(|candidate| candidate.get("status").and_then(Value::as_str))
        .collect();
    assert!(
        candidate_statuses.contains(&"WrittenOff"),
        "json missing written-off candidate: {candidate_statuses:?}"
    );
    assert!(
        candidates
            .iter()
            .filter_map(Value::as_object)
            .any(|candidate| candidate
                .get("write_off_reference")
                .and_then(Value::as_str)
                .is_some()),
        "json missing write-off reference metadata: {candidates:?}"
    );
    Ok(())
}

#[test]
fn ledger_reconciliation_summary_persists_telemetry() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut first_cmd = codex_command(codex_home.path())?;
    let first = first_cmd
        .args(["ledger", "reconciliation", "summary", "--format", "json"])
        .output()?;
    assert!(
        first.status.success(),
        "first reconciliation summary should succeed"
    );
    let first_value: Value = serde_json::from_slice(&first.stdout)?;
    let first_tx = first_value
        .get("telemetry")
        .and_then(Value::as_object)
        .and_then(|telemetry| telemetry.get("reconciliation_transactions"))
        .and_then(Value::as_u64)
        .expect("first run transactions");
    let mut second_cmd = codex_command(codex_home.path())?;
    let second = second_cmd
        .args(["ledger", "reconciliation", "summary", "--format", "json"])
        .output()?;
    assert!(
        second.status.success(),
        "second reconciliation summary should succeed"
    );
    let second_value: Value = serde_json::from_slice(&second.stdout)?;
    let second_tx = second_value
        .get("telemetry")
        .and_then(Value::as_object)
        .and_then(|telemetry| telemetry.get("reconciliation_transactions"))
        .and_then(Value::as_u64)
        .expect("second run transactions");
    assert!(
        second_tx > first_tx,
        "expected telemetry counters to increase: first={first_tx}, second={second_tx}"
    );
    Ok(())
}

#[test]
fn ledger_reconciliation_summary_handles_corrupt_telemetry_file() -> Result<()> {
    let codex_home = TempDir::new()?;
    let telemetry_path = codex_home.path().join("accounting/telemetry.json");
    if let Some(parent) = telemetry_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&telemetry_path, b"{not-json}")?;

    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd.args(["ledger", "reconciliation", "summary"]).output()?;
    assert!(
        output.status.success(),
        "reconciliation summary should succeed: {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("failed to load persisted telemetry"),
        "stderr missing telemetry warning: {stderr}"
    );
    let persisted = fs::read_to_string(&telemetry_path)?;
    serde_json::from_str::<Value>(&persisted)?;
    Ok(())
}

#[test]
fn ledger_go_live_checklist_surfaces_counters() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd.args(["ledger", "go-live-checklist"]).output()?;
    assert!(output.status.success(), "go-live checklist should succeed");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("Go-live readiness checklist:"),
        "stdout missing header: {stdout}"
    );
    assert!(
        stdout.contains("Period lock history"),
        "stdout missing period lock summary: {stdout}"
    );
    assert!(
        stdout.contains("Reconciliation coverage"),
        "stdout missing reconciliation coverage: {stdout}"
    );
    assert!(
        stdout.contains("Approvals backlog"),
        "stdout missing approvals line: {stdout}"
    );
    assert!(
        stdout.contains("run `codex ledger entries --format json`"),
        "stdout missing export validation pointer: {stdout}"
    );
    assert!(
        stdout.contains("Monitoring stubs"),
        "stdout missing monitoring stub note: {stdout}"
    );
    assert!(
        stdout.contains("Monitoring TODOs"),
        "stdout missing monitoring TODO placeholder: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry reset"),
        "stdout missing telemetry reset reminder: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry file:"),
        "stdout missing telemetry persistence note: {stdout}"
    );
    assert!(
        stdout.contains("Alert integration"),
        "stdout missing alert integration placeholder: {stdout}"
    );
    Ok(())
}

#[test]
fn tenancy_list_streams_reconciliation_metrics() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args([
            "tenancy",
            "list",
            "--firm-id",
            "demo-firm",
            "--stream-reconciliation",
        ])
        .output()?;
    assert!(output.status.success(), "tenancy list should succeed");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("No companies found for firm demo-firm."),
        "stdout missing empty firm message: {stdout}"
    );
    assert!(
        stdout.contains("Reconciliation metrics (demo feed):"),
        "stdout missing reconciliation header: {stdout}"
    );
    assert!(
        stdout.contains("tick 3"),
        "stdout missing third tick (ensures streaming loop ran): {stdout}"
    );
    assert!(
        stdout.contains("telemetry tx"),
        "stdout missing telemetry detail line: {stdout}"
    );
    assert!(
        stdout.contains("Telemetry file:"),
        "stdout missing telemetry file path: {stdout}"
    );
    Ok(())
}

#[test]
fn tenancy_list_streams_reconciliation_json() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args([
            "tenancy",
            "list",
            "--firm-id",
            "demo-firm",
            "--stream-reconciliation",
            "--json",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "tenancy list json should succeed: {:?}",
        output.status.code()
    );
    let stdout = String::from_utf8(output.stdout)?;
    let json_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.starts_with('{'))
        .collect();
    assert_eq!(
        json_lines.len(),
        2,
        "expected two JSON tick snapshots: {stdout}"
    );
    let ticks: Vec<Value> = json_lines
        .iter()
        .map(|line| serde_json::from_str::<Value>(line).expect("tick json"))
        .collect();
    let first = ticks
        .first()
        .expect("first tick")
        .get("tick")
        .and_then(Value::as_u64)
        .expect("tick number");
    assert_eq!(first, 1, "first tick index mismatch: {ticks:?}");
    let second = ticks
        .get(1)
        .expect("second tick")
        .get("tick")
        .and_then(Value::as_u64)
        .expect("tick number");
    assert_eq!(second, 2, "second tick index mismatch: {ticks:?}");
    let telemetry_path = ticks.iter().find_map(|tick| {
        tick.get("telemetry_path")
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    assert!(
        telemetry_path
            .as_ref()
            .is_some_and(|path| path.ends_with("telemetry.json")),
        "json missing telemetry path: {ticks:?}"
    );
    let approvals_totals: Vec<u64> = ticks
        .iter()
        .map(|tick| {
            tick.get("approvals")
                .and_then(|obj| obj.get("total"))
                .and_then(Value::as_u64)
                .expect("approvals total")
        })
        .collect();
    assert!(
        approvals_totals.iter().any(|total| *total > 0),
        "expected approvals backlog totals to surface: {ticks:?}"
    );
    let ingest = ticks
        .first()
        .and_then(|tick| tick.get("ingest"))
        .and_then(Value::as_object)
        .expect("ingest object");
    assert!(
        ingest
            .get("deduped_total")
            .and_then(Value::as_u64)
            .is_some_and(|value| value > 0),
        "ingest dedupe metrics missing: {ticks:?}"
    );
    let telemetry = ticks
        .first()
        .and_then(|tick| tick.get("telemetry"))
        .and_then(Value::as_object)
        .expect("telemetry object");
    telemetry
        .get("reconciliation_transactions")
        .and_then(Value::as_u64)
        .expect("reconciliation transactions counter missing");
    assert!(
        telemetry.contains_key("reconciliation_candidates"),
        "telemetry payload missing candidates counter: {ticks:?}"
    );
    if let Some(path) = telemetry_path {
        assert!(
            Path::new(&path).exists(),
            "telemetry path missing on disk: {path}"
        );
    }
    Ok(())
}
