use std::fs;
use std::sync::Arc;

use crate::reconciliation_output::ApprovalsBacklogOutput;
use crate::reconciliation_output::IngestSnapshotOutput;
use crate::reconciliation_output::ReconciliationTelemetryOutput;
use anyhow::Result;
use anyhow::anyhow;
use chrono::DateTime;
use chrono::Utc;
use clap::Parser;
use clap::ValueEnum;
use codex_accounting_api::AccountingTelemetry;
use codex_accounting_api::LedgerFacade;
use codex_accounting_api::TelemetryCounters;
use codex_accounting_api::demo::DemoLedgerData;
use codex_accounting_api::demo::demo_company_tenant;
use codex_accounting_api::demo::seed_demo_ledger;
use codex_accounting_api::demo::seed_demo_reconciliation;
use codex_accounting_api::duplicate_set_labels;
use codex_accounting_api::preview_copy::duplicate_guidance_message;
use codex_app_server_protocol::LedgerCompany;
use codex_app_server_protocol::LedgerLockPeriodParams;
use codex_app_server_protocol::LedgerPeriodAction;
use codex_app_server_protocol::LedgerPeriodRef;
use codex_common::CliConfigOverrides;
use codex_core::config::find_codex_home;
use codex_ledger::EnsurePeriodRequest;
use codex_ledger::InMemoryLedgerService;
use codex_ledger::LedgerService;
use codex_ledger::PeriodRef;
use serde::Serialize;

#[derive(Debug, Parser)]
pub struct LedgerCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    command: LedgerSubcommand,
}

#[derive(Debug, clap::Subcommand)]
enum LedgerSubcommand {
    /// Bootstrap a local demo ledger and post the first transaction.
    Demo,

    /// List seeded companies available in the demo ledger.
    Companies,

    /// List chart of accounts for a company (defaults to the seeded company).
    Accounts {
        /// Optional company identifier to filter results.
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: Option<String>,
    },

    /// Show recent journal entries for a company.
    Entries {
        /// Optional company identifier to filter results.
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: Option<String>,

        /// Maximum number of entries to display.
        #[arg(long = "limit", value_name = "COUNT", default_value_t = 5)]
        limit: usize,
    },

    /// List lock history for a journal period.
    ListLocks {
        /// Optional company identifier to filter results.
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: Option<String>,
        /// Journal identifier (defaults to the general ledger).
        #[arg(
            long = "journal-id",
            value_name = "JOURNAL_ID",
            default_value = "jnl-gl"
        )]
        journal_id: String,
        /// Output format (defaults to text).
        #[arg(long = "format", value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Set the lock state for a journal period.
    SetLock {
        /// Optional company identifier to filter results.
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: Option<String>,
        /// Journal identifier (defaults to the general ledger).
        #[arg(
            long = "journal-id",
            value_name = "JOURNAL_ID",
            default_value = "jnl-gl"
        )]
        journal_id: String,
        /// Fiscal year of the period.
        #[arg(long = "fiscal-year", value_name = "YEAR")]
        fiscal_year: i32,
        /// Period number within the fiscal year.
        #[arg(long = "period", value_name = "PERIOD")]
        period: u8,
        /// Requested lock action.
        #[arg(long = "action", value_enum)]
        action: LockAction,
        /// Required approval reference for auditing.
        #[arg(long = "approval-ref", value_name = "APPROVAL_REF")]
        approval_reference: String,
        /// Output format (defaults to text).
        #[arg(long = "format", value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Reconciliation helpers.
    Reconciliation {
        #[command(subcommand)]
        command: LedgerReconciliationSubcommand,
    },

    /// Run the go-live readiness checklist across demo services.
    GoLiveChecklist,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LockAction {
    SoftClose,
    Close,
    ReopenSoft,
    ReopenFull,
}

#[derive(Debug, clap::Subcommand)]
enum LedgerReconciliationSubcommand {
    /// Summarize reconciliation metrics for the demo dataset.
    Summary {
        /// Output format (defaults to text).
        #[arg(long = "format", value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

struct DemoLedgerContext {
    data: DemoLedgerData,
    facade: LedgerFacade,
    telemetry: Arc<AccountingTelemetry>,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl OutputFormat {
    #[must_use]
    fn is_json(self) -> bool {
        matches!(self, OutputFormat::Json)
    }
}

pub async fn run(cli: LedgerCli) -> Result<()> {
    // Validate shared overrides even though the demo ledger currently ignores them.
    let _ = cli
        .config_overrides
        .parse_overrides()
        .map_err(|err| anyhow!(err))?;

    match cli.command {
        LedgerSubcommand::Demo => run_demo().await,
        LedgerSubcommand::Companies => run_companies().await,
        LedgerSubcommand::Accounts { company_id } => run_accounts(company_id).await,
        LedgerSubcommand::Entries { company_id, limit } => run_entries(company_id, limit).await,
        LedgerSubcommand::ListLocks {
            company_id,
            journal_id,
            format,
        } => run_list_locks(company_id, journal_id, format).await,
        LedgerSubcommand::SetLock {
            company_id,
            journal_id,
            fiscal_year,
            period,
            action,
            approval_reference,
            format,
        } => {
            run_set_lock(
                company_id,
                journal_id,
                fiscal_year,
                period,
                action,
                approval_reference,
                format,
            )
            .await
        }
        LedgerSubcommand::Reconciliation { command } => match command {
            LedgerReconciliationSubcommand::Summary { format } => {
                run_reconciliation_summary(format).await
            }
        },
        LedgerSubcommand::GoLiveChecklist => run_go_live_checklist().await,
    }
}

async fn run_demo() -> Result<()> {
    let DemoLedgerContext { data, .. } = build_demo_context().await?;
    let company = data
        .companies
        .first()
        .ok_or_else(|| anyhow!("Demo ledger failed to seed a company"))?;
    let accounts = data.accounts.iter().collect::<Vec<_>>();
    let entry = data
        .entries
        .first()
        .ok_or_else(|| anyhow!("Demo ledger failed to seed an entry"))?;

    println!("Created company {} ({})", company.name, company.id);
    match accounts.as_slice() {
        [first, second, ..] => {
            println!("Configured accounts: {} • {}", first.code, second.code);
        }
        [first] => {
            println!("Configured account {}", first.code);
        }
        _ => println!("No demo accounts were seeded."),
    }
    println!(
        "Posted entry {} to journal {} with {} lines",
        entry.entry.id,
        entry.entry.journal_id,
        entry.entry.lines.len()
    );
    println!("Ledger demo complete. Use F6 in the TUI to preview upcoming workflows.");

    Ok(())
}

async fn run_companies() -> Result<()> {
    let DemoLedgerContext { data, .. } = build_demo_context().await?;
    if data.companies.is_empty() {
        println!("No companies are available in the demo ledger.");
        return Ok(());
    }

    println!("Companies");
    for company in &data.companies {
        println!(
            "- {} ({}) • base currency {}",
            company.name, company.id, company.base_currency.code
        );
    }

    Ok(())
}

async fn run_accounts(company_id: Option<String>) -> Result<()> {
    let DemoLedgerContext { data, .. } = build_demo_context().await?;
    let company = resolve_company(&data.companies, company_id.as_deref())?;

    let accounts: Vec<_> = data
        .accounts
        .iter()
        .filter(|account| account.company_id == company.id)
        .collect();

    if accounts.is_empty() {
        println!("No accounts found for {} ({})", company.name, company.id);
        return Ok(());
    }

    println!("Accounts for {} ({})", company.name, company.id);
    for account in accounts {
        println!(
            "- {} ({}) • {} • {:?}",
            account.code, account.id, account.name, account.account_type
        );
    }

    Ok(())
}

async fn run_entries(company_id: Option<String>, limit: usize) -> Result<()> {
    let DemoLedgerContext { data, .. } = build_demo_context().await?;
    let company = resolve_company(&data.companies, company_id.as_deref())?;

    let entries: Vec<_> = data
        .entries
        .iter()
        .filter(|entry| entry.company_id == company.id)
        .collect();

    if entries.is_empty() {
        println!(
            "No journal entries found for {} ({})",
            company.name, company.id
        );
        return Ok(());
    }

    println!(
        "Recent journal entries for {} ({})",
        company.name, company.id
    );
    for entry in entries.into_iter().take(limit) {
        println!(
            "- {} • journal {} • {:?} • {} lines",
            entry.entry.id,
            entry.entry.journal_id,
            entry.entry.status,
            entry.entry.lines.len()
        );
    }

    Ok(())
}

async fn run_list_locks(
    company_id: Option<String>,
    journal_id: String,
    format: OutputFormat,
) -> Result<()> {
    let DemoLedgerContext {
        data,
        facade,
        telemetry,
    } = build_demo_context().await?;
    let company = resolve_company(&data.companies, company_id.as_deref())?;
    seed_sample_locks(&facade, &company.id, &journal_id).await?;

    let journal = facade
        .ensure_period(EnsurePeriodRequest {
            journal_id: journal_id.clone(),
            period: PeriodRef {
                fiscal_year: 2024,
                period: 3,
            },
            tenant: demo_company_tenant(&company.id),
        })
        .await
        .map_err(|err| anyhow!(err))?;

    let counters = telemetry.snapshot();

    if format.is_json() {
        let locks = journal
            .lock_history()
            .iter()
            .map(|lock| {
                let locked_at: DateTime<Utc> = lock.locked_at.into();
                LockHistoryEntryOutput {
                    fiscal_year: lock.period.fiscal_year,
                    period: lock.period.period,
                    action: format!("{:?}", lock.action),
                    approval_reference: lock.approval_reference.clone(),
                    locked_at: locked_at.to_rfc3339(),
                    locked_by: lock.locked_by.clone(),
                }
            })
            .collect();
        let telemetry_path = telemetry
            .store_path()
            .map(|path| path.display().to_string());
        let payload = LockHistoryOutput {
            company_id: company.id.clone(),
            company_name: company.name.clone(),
            journal_id: journal_id.clone(),
            locks,
            telemetry: LockTelemetryOutput::from(&counters),
            telemetry_path,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "Lock history for {} ({}) on journal {}",
        company.name, company.id, journal_id
    );
    if journal.lock_history().is_empty() {
        println!("No lock events recorded yet.");
        return Ok(());
    }
    for lock in journal.lock_history() {
        let timestamp: DateTime<Utc> = lock.locked_at.into();
        let approval = lock.approval_reference.as_deref().unwrap_or("n/a");
        println!(
            "- {}/{} • {:?} • approval {} • {} by {}",
            lock.period.fiscal_year,
            lock.period.period,
            lock.action,
            approval,
            format_since(timestamp),
            lock.locked_by
        );
    }

    println!(
        "Telemetry: {} lock events ({} close / {} soft / {} reopen).",
        counters.period_lock_events,
        counters.period_lock_close,
        counters.period_lock_soft_close,
        counters.period_lock_reopen_soft + counters.period_lock_reopen_full
    );
    match telemetry.store_path() {
        Some(path) => println!("Telemetry file: {}", path.display()),
        None => println!("Telemetry file: in-memory (set CODEX_HOME to persist)."),
    }

    Ok(())
}

async fn run_set_lock(
    company_id: Option<String>,
    journal_id: String,
    fiscal_year: i32,
    period: u8,
    action: LockAction,
    approval_reference: String,
    format: OutputFormat,
) -> Result<()> {
    if approval_reference.trim().is_empty() {
        anyhow::bail!("approval reference must be provided");
    }
    let DemoLedgerContext {
        data,
        facade,
        telemetry,
    } = build_demo_context().await?;
    let company = resolve_company(&data.companies, company_id.as_deref())?;
    let params = LedgerLockPeriodParams {
        company_id: company.id.clone(),
        journal_id: journal_id.clone(),
        period: LedgerPeriodRef {
            fiscal_year,
            period,
        },
        action: map_lock_action(action),
        approval_reference: Some(approval_reference.clone()),
    };
    let response = facade
        .lock_period(params, demo_company_tenant(&company.id))
        .await
        .map_err(|err| anyhow!(err))?;

    let latest = response
        .journal
        .latest_lock
        .ok_or_else(|| anyhow!("lock operation did not record history"))?;
    let counters = telemetry.snapshot();

    if format.is_json() {
        let telemetry_path = telemetry
            .store_path()
            .map(|path| path.display().to_string());
        let payload = LockUpdateOutput {
            journal_id: journal_id.clone(),
            period: LockPeriodOutput {
                fiscal_year: latest.period.fiscal_year,
                period: latest.period.period,
            },
            action: format!("{:?}", latest.action),
            approval_reference: latest
                .approval_reference
                .clone()
                .unwrap_or_else(|| approval_reference.clone()),
            locked_at: latest.locked_at.clone(),
            locked_by: latest.locked_by,
            telemetry: LockTelemetryOutput::from(&counters),
            telemetry_path,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let timestamp = DateTime::parse_from_rfc3339(&latest.locked_at)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    println!(
        "Updated journal {} period {}/{} to {:?} (approval {} • {} by {})",
        journal_id,
        latest.period.fiscal_year,
        latest.period.period,
        latest.action,
        approval_reference,
        format_since(timestamp),
        latest.locked_by
    );

    println!(
        "Telemetry: {} lock events recorded ({} close / {} soft / {} reopen).",
        counters.period_lock_events,
        counters.period_lock_close,
        counters.period_lock_soft_close,
        counters.period_lock_reopen_soft + counters.period_lock_reopen_full
    );
    match telemetry.store_path() {
        Some(path) => println!("Telemetry file: {}", path.display()),
        None => println!("Telemetry file: in-memory (set CODEX_HOME to persist)."),
    }

    Ok(())
}

async fn run_reconciliation_summary(format: OutputFormat) -> Result<()> {
    warn_if_corrupt_telemetry();
    let context = seed_demo_reconciliation()
        .await
        .map_err(|err| anyhow!(err))?;
    let company_name = context
        .ledger
        .companies
        .first()
        .map(|company| company.name.clone())
        .unwrap_or_else(|| "Demo company".to_string());
    let summary = context
        .facade
        .summary(&context.company_id)
        .map_err(|err| anyhow!(err))?;
    let transactions = context
        .facade
        .list_transactions(&context.company_id)
        .map_err(|err| anyhow!(err))?;
    let duplicate_sets = duplicate_set_labels(&transactions);
    let candidates = context
        .facade
        .list_candidates(&context.session_id)
        .map_err(|err| anyhow!(err))?;
    let counters = context.telemetry.snapshot();

    if format.is_json() {
        let telemetry_path = context
            .telemetry
            .store_path()
            .map(|path| path.display().to_string());
        let payload = ReconciliationSummaryOutput {
            company_id: context.company_id.clone(),
            company_name,
            matched: summary.matched,
            pending: summary.pending,
            coverage_ratio: summary.coverage_ratio(),
            ingest: IngestSnapshotOutput {
                ingested_total: context.ingest_snapshot.ingested_total,
                deduped_total: context.ingest_snapshot.deduped_total,
                duplicates_dropped: context.ingest_snapshot.duplicates_dropped,
                last_feed_at: context.ingest_snapshot.last_ingest_at.to_rfc3339(),
            },
            approvals: ApprovalsBacklogOutput {
                generated_at: context.approvals_view.generated_at.to_rfc3339(),
                total: context.approvals_view.tasks.len(),
                overdue: context.approvals_view.overdue.len(),
            },
            telemetry_path,
            transactions: transactions
                .iter()
                .map(|tx| TransactionOutput {
                    transaction_id: tx.transaction_id.clone(),
                    posted_date: tx.posted_date.to_string(),
                    description: tx.description.clone(),
                    amount_minor: tx.amount_minor,
                    currency: tx.currency.clone(),
                    account_id: tx.account_id.clone(),
                    source_reference: tx.source_reference.clone(),
                    source_checksum: tx.source_checksum.clone(),
                    is_void: tx.is_void,
                    duplicates_dropped: tx.duplicate_metadata.total_occurrences.saturating_sub(1),
                    duplicate_group: tx.duplicate_metadata.group_key.clone(),
                })
                .collect(),
            candidates: candidates
                .iter()
                .map(|candidate| CandidateOutput {
                    transaction_id: candidate.transaction_id.clone(),
                    journal_entry_id: candidate.journal_entry_id.clone(),
                    status: format!("{:?}", candidate.status),
                    score: candidate.score,
                    group_id: candidate.group_id.clone(),
                    write_off_reference: candidate.write_off_reason.clone(),
                })
                .collect(),
            telemetry: ReconciliationTelemetryOutput::from(&counters),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!(
        "Reconciliation summary for {} ({})",
        company_name, context.company_id
    );
    println!(
        "- Coverage {:.0}% (matched {} • pending {})",
        (summary.coverage_ratio() * 100.0).clamp(0.0, 100.0),
        summary.matched,
        summary.pending
    );
    if !duplicate_sets.is_empty() {
        print_duplicate_guidance(&duplicate_sets);
    }
    println!(
        "- Ingest deduped {} of {} (last feed {})",
        context.ingest_snapshot.deduped_total,
        context.ingest_snapshot.ingested_total,
        format_since(context.ingest_snapshot.last_ingest_at)
    );
    if !duplicate_sets.is_empty() {
        print_duplicate_guidance(&duplicate_sets);
    }
    println!(
        "- Approvals backlog: {} overdue / {} open (generated {})",
        context.approvals_view.overdue.len(),
        context.approvals_view.tasks.len(),
        context
            .approvals_view
            .generated_at
            .format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("- Recent bank transactions:");
    for tx in transactions.iter().take(3) {
        println!(
            "  • {} • {} • {}",
            tx.posted_date,
            tx.description,
            format_currency(tx.amount_minor)
        );
    }
    println!("- Match candidates:");
    for candidate in candidates.iter().take(3) {
        println!(
            "  • {} ⇔ {} • {:?} • score {:.0}%",
            candidate.transaction_id,
            candidate.journal_entry_id,
            candidate.status,
            (candidate.score * 100.0).clamp(0.0, 100.0)
        );
    }

    println!(
        "- Telemetry counters: {} transactions • {} candidates • {} write-offs",
        counters.reconciliation_transactions,
        counters.reconciliation_candidates,
        counters.reconciliation_write_offs
    );
    println!(
        "  Period locks: {} total (close {} / soft {} / reopen {})",
        counters.period_lock_events,
        counters.period_lock_close,
        counters.period_lock_soft_close,
        counters.period_lock_reopen_soft + counters.period_lock_reopen_full
    );
    println!(
        "  Policy decisions: auto-post {} • needs-approval {} • reject {}",
        counters.policy_auto_post, counters.policy_needs_approval, counters.policy_reject
    );
    println!(
        "  Approvals snapshot: {} open / {} overdue",
        counters.approvals_total, counters.approvals_overdue
    );
    match context.telemetry.store_path() {
        Some(path) => println!("- Telemetry file: {}", path.display()),
        None => println!("- Telemetry file: in-memory (set CODEX_HOME to persist)."),
    }

    Ok(())
}

fn warn_if_corrupt_telemetry() {
    if let Ok(mut home) = find_codex_home() {
        home.push("accounting");
        home.push("telemetry.json");
        if !home.exists() {
            return;
        }
        match fs::read(&home) {
            Ok(data) => {
                if let Err(err) = serde_json::from_slice::<TelemetryCounters>(&data) {
                    eprintln!(
                        "failed to load persisted telemetry; continuing with defaults ({}): {err}",
                        home.display()
                    );
                }
            }
            Err(err) => {
                eprintln!(
                    "failed to load persisted telemetry; continuing with defaults ({}): {err}",
                    home.display()
                );
            }
        }
    }
}

fn print_duplicate_guidance(duplicate_sets: &[String]) {
    for label in duplicate_sets {
        println!("  • {}", duplicate_guidance_message(label));
    }
}

async fn run_go_live_checklist() -> Result<()> {
    let DemoLedgerContext {
        data,
        facade,
        telemetry,
    } = build_demo_context().await?;
    let company = resolve_company(&data.companies, None)?;
    seed_sample_locks(&facade, &company.id, "jnl-gl").await?;
    let lock_counters = telemetry.snapshot();

    let reconciliation = seed_demo_reconciliation()
        .await
        .map_err(|err| anyhow!(err))?;
    let summary = reconciliation
        .facade
        .summary(&reconciliation.company_id)
        .map_err(|err| anyhow!(err))?;
    let approvals_overdue = reconciliation.approvals_view.overdue.len();
    let approvals_total = reconciliation.approvals_view.tasks.len();

    println!("Go-live readiness checklist:");
    println!(
        "- Period lock history: {} events (close {} / soft {} / reopen {}).",
        lock_counters.period_lock_events,
        lock_counters.period_lock_close,
        lock_counters.period_lock_soft_close,
        lock_counters.period_lock_reopen_soft + lock_counters.period_lock_reopen_full
    );
    println!(
        "- Reconciliation coverage: {:.0}% ({} matched / {} pending).",
        (summary.coverage_ratio() * 100.0).clamp(0.0, 100.0),
        summary.matched,
        summary.pending
    );
    if approvals_total == 0 {
        println!("- Approvals backlog: queue empty ✔");
    } else {
        let status = if approvals_overdue == 0 { "✔" } else { "⚠" };
        println!(
            "- Approvals backlog: {approvals_overdue} overdue / {approvals_total} open {status}"
        );
    }
    println!(
        "- Export validation: run `codex ledger entries --format json` to diff exports; {} journal entries balanced.",
        data.entries.len()
    );

    let next_sla = reconciliation
        .approvals_view
        .tasks
        .iter()
        .filter_map(|task| task.request.sla_at)
        .min();
    if let Some(deadline) = next_sla {
        println!(
            "- Monitoring stubs: telemetry.period_lock_events={} • next SLA deadline {}.",
            lock_counters.period_lock_events,
            deadline.format("%Y-%m-%d %H:%M:%S UTC")
        );
    } else {
        println!(
            "- Monitoring stubs: telemetry.period_lock_events={} • SLA queue empty ✔",
            lock_counters.period_lock_events
        );
    }
    println!("- Monitoring TODOs: connect metrics dashboards and pager alerts before launch.");
    println!(
        "- Alert integration: placeholder hooks remain for paging/monitoring; wire to incident tooling before pilot."
    );
    match reconciliation.telemetry.store_path() {
        Some(path) => {
            println!("- Telemetry file: {}", path.display());
            println!(
                "- Telemetry reset: delete {} before each dry run to clear counters.",
                path.display()
            );
        }
        None => {
            println!("- Telemetry file: in-memory (set CODEX_HOME to persist).");
            println!("- Telemetry reset: restart the CLI to clear in-memory counters.");
        }
    }

    Ok(())
}

async fn build_demo_context() -> Result<DemoLedgerContext> {
    let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
    let telemetry = Arc::new(AccountingTelemetry::persistent_from_env());
    let facade = LedgerFacade::with_telemetry(service, Some(telemetry.clone()));
    let data = seed_demo_ledger(&facade)
        .await
        .map_err(|err| anyhow!(err))?;
    Ok(DemoLedgerContext {
        data,
        facade,
        telemetry,
    })
}

async fn seed_sample_locks(
    facade: &LedgerFacade,
    company_id: &str,
    journal_id: &str,
) -> Result<()> {
    let samples = [
        (2024, 1, LedgerPeriodAction::SoftClose, "APR-LCK-001"),
        (2024, 1, LedgerPeriodAction::Close, "APR-LCK-002"),
        (2024, 2, LedgerPeriodAction::SoftClose, "APR-LCK-003"),
    ];
    for (year, period, action, approval) in samples {
        let params = LedgerLockPeriodParams {
            company_id: company_id.to_string(),
            journal_id: journal_id.to_string(),
            period: LedgerPeriodRef {
                fiscal_year: year,
                period,
            },
            action,
            approval_reference: Some(approval.to_string()),
        };
        let _ = facade
            .lock_period(params, demo_company_tenant(company_id))
            .await
            .map_err(|err| anyhow!(err))?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct LockHistoryOutput {
    company_id: String,
    company_name: String,
    journal_id: String,
    locks: Vec<LockHistoryEntryOutput>,
    telemetry: LockTelemetryOutput,
    telemetry_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct LockHistoryEntryOutput {
    fiscal_year: i32,
    period: u8,
    action: String,
    approval_reference: Option<String>,
    locked_at: String,
    locked_by: String,
}

#[derive(Debug, Serialize)]
struct LockTelemetryOutput {
    events: usize,
    soft_close: usize,
    close: usize,
    reopen_soft: usize,
    reopen_full: usize,
}

impl From<&TelemetryCounters> for LockTelemetryOutput {
    fn from(counters: &TelemetryCounters) -> Self {
        Self {
            events: counters.period_lock_events,
            soft_close: counters.period_lock_soft_close,
            close: counters.period_lock_close,
            reopen_soft: counters.period_lock_reopen_soft,
            reopen_full: counters.period_lock_reopen_full,
        }
    }
}

#[derive(Debug, Serialize)]
struct LockUpdateOutput {
    journal_id: String,
    period: LockPeriodOutput,
    action: String,
    approval_reference: String,
    locked_at: String,
    locked_by: String,
    telemetry: LockTelemetryOutput,
    telemetry_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct LockPeriodOutput {
    fiscal_year: i32,
    period: u8,
}

#[derive(Debug, Serialize)]
struct ReconciliationSummaryOutput {
    company_id: String,
    company_name: String,
    matched: usize,
    pending: usize,
    coverage_ratio: f32,
    ingest: IngestSnapshotOutput,
    approvals: ApprovalsBacklogOutput,
    telemetry_path: Option<String>,
    transactions: Vec<TransactionOutput>,
    candidates: Vec<CandidateOutput>,
    telemetry: ReconciliationTelemetryOutput,
}

#[derive(Debug, Serialize)]
struct TransactionOutput {
    transaction_id: String,
    posted_date: String,
    description: String,
    amount_minor: i64,
    currency: String,
    account_id: String,
    source_reference: Option<String>,
    source_checksum: Option<String>,
    is_void: bool,
    duplicates_dropped: usize,
    duplicate_group: Option<String>,
}

#[derive(Debug, Serialize)]
struct CandidateOutput {
    transaction_id: String,
    journal_entry_id: String,
    status: String,
    score: f32,
    group_id: Option<String>,
    write_off_reference: Option<String>,
}

fn map_lock_action(action: LockAction) -> LedgerPeriodAction {
    match action {
        LockAction::SoftClose => LedgerPeriodAction::SoftClose,
        LockAction::Close => LedgerPeriodAction::Close,
        LockAction::ReopenSoft => LedgerPeriodAction::ReopenSoft,
        LockAction::ReopenFull => LedgerPeriodAction::ReopenFull,
    }
}

fn format_currency(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let value = amount_minor.abs();
    let units = value / 100;
    let cents = value % 100;
    format!("{sign}${units}.{cents:02}")
}

fn format_since(timestamp: DateTime<Utc>) -> String {
    let delta = Utc::now() - timestamp;
    let seconds = delta.num_seconds().abs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        if minutes > 0 {
            format!("{hours}h {minutes:02}m ago")
        } else {
            format!("{hours}h ago")
        }
    } else if minutes > 0 {
        format!("{minutes}m ago")
    } else {
        format!("{seconds}s ago")
    }
}

fn resolve_company<'a>(
    companies: &'a [LedgerCompany],
    selector: Option<&str>,
) -> Result<&'a LedgerCompany> {
    match selector {
        Some(company_id) => companies
            .iter()
            .find(|company| company.id == company_id)
            .ok_or_else(|| anyhow!("Company {company_id} not found")),
        None => companies
            .first()
            .ok_or_else(|| anyhow!("No companies are available in the demo ledger")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn currency_formatting_is_stable() {
        assert_eq!(format_currency(12_500), "$125.00");
        assert_eq!(format_currency(-305), "-$3.05");
    }

    #[test]
    fn lock_action_mapping_preserves_variants() {
        assert!(matches!(
            map_lock_action(LockAction::Close),
            LedgerPeriodAction::Close
        ));
        assert!(matches!(
            map_lock_action(LockAction::SoftClose),
            LedgerPeriodAction::SoftClose
        ));
    }

    #[test]
    fn set_lock_rejects_blank_approval() {
        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let result = run_set_lock(
                None,
                "jnl-gl".to_string(),
                2024,
                1,
                LockAction::Close,
                "  ".to_string(),
                OutputFormat::Text,
            )
            .await;
            assert!(result.is_err());
        });
    }
}
