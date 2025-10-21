use std::path::Path;

use anyhow::Result;
use assert_cmd::Command;
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
    Ok(())
}

#[test]
fn ledger_reconciliation_summary_highlights_metrics() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = codex_command(codex_home.path())?;
    let output = cmd
        .args(["ledger", "reconciliation", "summary"])
        .output()?;
    assert!(output.status.success(), "reconciliation summary should succeed");
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
    Ok(())
}
