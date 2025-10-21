use std::cmp::max;
use std::collections::HashSet;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::reconciliation_output::ApprovalsBacklogOutput;
use crate::reconciliation_output::IngestSnapshotOutput;
use crate::reconciliation_output::ReconciliationStreamTickOutput;
use crate::reconciliation_output::ReconciliationTelemetryOutput;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use chrono::DateTime;
use chrono::Utc;
use clap::Parser;
use codex_accounting_api::AccountingTelemetry;
use codex_accounting_api::ApprovalsQueueView;
use codex_accounting_api::ControlsFacade;
use codex_accounting_api::TelemetryCounters;
use codex_accounting_api::TenancyFacade;
use codex_accounting_api::demo::seed_demo_reconciliation;
use codex_approvals::ApprovalStatus;
use codex_approvals::ApprovalTask;
use codex_approvals::InMemoryApprovalsService;
use codex_approvals::QueueFilter;
use codex_common::CliConfigOverrides;
use codex_core::config::find_codex_home;
use codex_ledger::AccountType as LedgerAccountType;
use codex_ledger::ChartAccount;
use codex_ledger::CreateCompanyRequest as LedgerCreateCompanyRequest;
use codex_ledger::Currency as LedgerCurrency;
use codex_ledger::CurrencyMode as LedgerCurrencyMode;
use codex_ledger::FiscalCalendar as LedgerFiscalCalendar;
use codex_ledger::InMemoryLedgerService;
use codex_ledger::LedgerService;
use codex_ledger::Role as LedgerRole;
use codex_ledger::SeedChartRequest;
use codex_ledger::TenantContext as LedgerTenantContext;
use codex_policy::EvaluationOutcome;
use codex_policy::InMemoryPolicyStore;
use codex_policy::PolicyContext;
use codex_policy::PolicyDecision;
use codex_policy::PolicyEngine;
use codex_policy::PolicyEventSink;
use codex_policy::PolicyRuleSet;
use codex_policy::PolicyTrigger;
use codex_policy::PostingProposal;
use codex_tenancy::Company;
use codex_tenancy::CompanyStatus;
use codex_tenancy::CreateCompanyRequest;
use codex_tenancy::CreateFirmRequest;
use codex_tenancy::Firm;
use codex_tenancy::InMemoryTenancyService;
use codex_tenancy::Role;
use codex_tenancy::RoleAssignment;
use codex_tenancy::RoleScope;
use codex_tenancy::TenancyError;
use codex_tenancy::TenancyService;
use codex_tenancy::TenancySnapshot;
use codex_tenancy::UserAccount;
use codex_tenancy::UserStatus;
use serde::Deserialize;
use serde::Serialize;
use serde_json::to_string;
use tokio::time::Duration as TokioDuration;
use tokio::time::sleep;

#[derive(Debug, Parser)]
pub struct TenancyCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    command: TenancyCommand,
}

#[derive(Debug, clap::Subcommand)]
enum TenancyCommand {
    /// List companies for a firm.
    List {
        /// Firm identifier to scope results.
        #[arg(long = "firm-id", value_name = "FIRM_ID")]
        firm_id: String,
        /// Stream reconciliation metrics alongside the listing.
        #[arg(long = "stream-reconciliation")]
        stream_reconciliation: bool,
        /// Emit streamed reconciliation metrics as newline-delimited JSON.
        #[arg(
            long = "json",
            default_value_t = false,
            requires = "stream_reconciliation"
        )]
        json: bool,
    },
    /// Create a company within a firm.
    Create {
        /// Firm identifier that owns the company.
        #[arg(long = "firm-id", value_name = "FIRM_ID")]
        firm_id: String,
        /// Company display name.
        #[arg(long = "name", value_name = "NAME")]
        name: String,
        /// Base currency in ISO-4217 format.
        #[arg(long = "currency", value_name = "CURRENCY")]
        currency: String,
        /// Optional company tags. Repeat the flag to add multiple tags.
        #[arg(long = "tag", value_name = "TAG")]
        tags: Vec<String>,
        /// Optional metadata payload (opaque string for now).
        #[arg(long = "metadata", value_name = "METADATA")]
        metadata: Option<String>,
    },
    /// Archive a company.
    Archive {
        #[arg(long = "firm-id", value_name = "FIRM_ID")]
        firm_id: String,
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: String,
    },
    /// Reactivate a previously archived company.
    Reactivate {
        #[arg(long = "firm-id", value_name = "FIRM_ID")]
        firm_id: String,
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: String,
    },
    /// Manage policy configuration.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Inspect approvals queue state.
    Approvals {
        #[command(subcommand)]
        command: ApprovalsCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum PolicyCommand {
    /// List configured policy rules and preview triggers.
    Show {
        /// Filter to a single company (optional).
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: Option<String>,
    },
    /// Update policy thresholds and approval requirements for a company.
    Set {
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: String,
        #[arg(long = "auto-post-limit", value_name = "MAJOR_AMOUNT")]
        auto_post_limit: String,
        #[arg(long = "confidence-floor", value_name = "FRACTION")]
        confidence_floor: f32,
        #[arg(long = "require-account", value_name = "ACCOUNT")]
        require_accounts: Vec<String>,
        #[arg(long = "require-vendor", value_name = "VENDOR")]
        require_vendors: Vec<String>,
    },
}

#[derive(Debug, clap::Subcommand)]
enum ApprovalsCommand {
    /// Display pending approvals for a company.
    Queue {
        #[arg(long = "company-id", value_name = "COMPANY_ID")]
        company_id: String,
    },
}

pub async fn run(cli: TenancyCli) -> Result<()> {
    let _ = cli
        .config_overrides
        .parse_overrides()
        .map_err(|err| anyhow::anyhow!(err))?;

    let (store, service) = TenancyStore::load()?;
    let mut bootstrap = AccountingBootstrap::load()?;
    let tenancy_service: Arc<dyn TenancyService> = service.clone();
    let facade = TenancyFacade::new(tenancy_service);
    let mut dirty = false;
    let mut bootstrap_dirty = false;

    match cli.command {
        TenancyCommand::List {
            firm_id,
            stream_reconciliation,
            json,
        } => {
            let stream_format = if json {
                StreamFormat::Json
            } else {
                StreamFormat::Text
            };
            list_companies(
                &facade,
                &bootstrap,
                &firm_id,
                stream_reconciliation,
                stream_format,
            )
            .await?;
        }
        TenancyCommand::Create {
            firm_id,
            name,
            currency,
            tags,
            metadata,
        } => {
            let outcome = create_company(
                &facade,
                &mut bootstrap,
                firm_id,
                name,
                currency,
                tags,
                metadata,
            )
            .await?;
            if outcome {
                bootstrap_dirty = true;
            }
            dirty = true;
        }
        TenancyCommand::Archive {
            firm_id,
            company_id,
        } => {
            archive_company(&facade, firm_id, company_id).await?;
            dirty = true;
        }
        TenancyCommand::Reactivate {
            firm_id,
            company_id,
        } => {
            reactivate_company(&facade, firm_id, company_id).await?;
            dirty = true;
        }
        TenancyCommand::Policy { command } => {
            let policy_dirty = handle_policy_command(&facade, &mut bootstrap, command).await?;
            if policy_dirty {
                bootstrap_dirty = true;
            }
        }
        TenancyCommand::Approvals { command } => {
            handle_approvals_command(command).await?;
        }
    }

    if dirty {
        store.persist(service).await?;
    }
    if bootstrap_dirty {
        bootstrap.persist()?;
    }

    Ok(())
}

async fn list_companies(
    facade: &TenancyFacade,
    bootstrap: &AccountingBootstrap,
    firm_id: &str,
    stream_reconciliation: bool,
    stream_format: StreamFormat,
) -> Result<()> {
    let firm = firm_id.to_string();
    let companies = facade
        .list_companies(&firm)
        .await
        .context("failed to list companies")?;

    if companies.is_empty() {
        println!("No companies found for firm {firm_id}.");
        if stream_reconciliation {
            stream_reconciliation_metrics(stream_format).await?;
        }
        return Ok(());
    }

    println!("Companies for firm {firm_id}");
    for company in companies {
        let status = match company.status {
            CompanyStatus::Active => "active",
            CompanyStatus::Archived => "archived",
        };
        let tags = if company.tags.is_empty() {
            "-".to_string()
        } else {
            company.tags.join(", ")
        };
        println!(
            "- {} ({}) • {} • tags: {}",
            company.name, company.id, status, tags
        );
        if let Some(summary) = bootstrap.get(&company.id) {
            println!(
                "    ledger {} • policy {}",
                summary.ledger_company_id,
                policy_summary(&summary.policy_rules, &summary.base_currency)
            );
        }
    }

    if stream_reconciliation {
        stream_reconciliation_metrics(stream_format).await?;
    }

    Ok(())
}

async fn stream_reconciliation_metrics(format: StreamFormat) -> Result<()> {
    if !format.is_json() {
        println!("Reconciliation metrics (demo feed):");
    }
    let mut telemetry_path: Option<String> = None;
    let ticks = if format.is_json() { 2 } else { 3 };
    let delay = if format.is_json() {
        Some(TokioDuration::from_millis(100))
    } else {
        Some(TokioDuration::from_millis(500))
    };
    for tick in 0..ticks {
        let context = seed_demo_reconciliation()
            .await
            .map_err(|err| anyhow!(err))?;
        if telemetry_path.is_none() {
            telemetry_path = context
                .telemetry
                .store_path()
                .map(|path| path.display().to_string());
        }
        let summary = context
            .facade
            .summary(&context.company_id)
            .map_err(|err| anyhow!(err))?;
        let total = summary.matched + summary.pending;
        let coverage = if total == 0 {
            0.0
        } else {
            summary.matched as f32 / total as f32
        };
        let counters = context.telemetry.snapshot();
        if format.is_json() {
            let approvals = ApprovalsBacklogOutput {
                generated_at: context.approvals_view.generated_at.to_rfc3339(),
                total: context.approvals_view.tasks.len(),
                overdue: context.approvals_view.overdue.len(),
            };
            let ingest = IngestSnapshotOutput {
                ingested_total: context.ingest_snapshot.ingested_total,
                deduped_total: context.ingest_snapshot.deduped_total,
                duplicates_dropped: context.ingest_snapshot.duplicates_dropped,
                last_feed_at: context.ingest_snapshot.last_ingest_at.to_rfc3339(),
            };
            let payload = ReconciliationStreamTickOutput {
                tick: tick + 1,
                matched: summary.matched,
                pending: summary.pending,
                coverage_ratio: coverage,
                coverage_percent: (coverage * 100.0).clamp(0.0, 100.0),
                approvals,
                ingest,
                telemetry: ReconciliationTelemetryOutput::from(&counters),
                telemetry_path: telemetry_path.clone(),
                generated_at: Utc::now(),
            };
            println!("{}", to_string(&payload)?);
        } else {
            println!(
                "  tick {}: matched {} | pending {} | coverage {:.0}% | backlog {} overdue / {} open",
                tick + 1,
                summary.matched,
                summary.pending,
                (coverage * 100.0).clamp(0.0, 100.0),
                context.approvals_view.overdue.len(),
                context.approvals_view.tasks.len()
            );
            println!(
                "            ingest {}/{} deduped • last feed {}",
                context.ingest_snapshot.deduped_total,
                context.ingest_snapshot.ingested_total,
                short_since(context.ingest_snapshot.last_ingest_at)
            );
            println!(
                "            telemetry tx {} • candidates {} • write-offs {}",
                counters.reconciliation_transactions,
                counters.reconciliation_candidates,
                counters.reconciliation_write_offs
            );
        }
        if tick + 1 < ticks
            && let Some(duration) = delay
        {
            sleep(duration).await;
        }
    }
    if format.is_json() {
        return Ok(());
    }
    match telemetry_path {
        Some(path) => println!("Telemetry file: {path}"),
        None => println!("Telemetry file: in-memory (set CODEX_HOME to persist)."),
    }
    Ok(())
}

fn short_since(timestamp: DateTime<Utc>) -> String {
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

async fn create_company(
    facade: &TenancyFacade,
    bootstrap: &mut AccountingBootstrap,
    firm_id: String,
    name: String,
    currency: String,
    tags: Vec<String>,
    metadata: Option<String>,
) -> Result<bool> {
    ensure_firm_exists(facade, &firm_id)
        .await
        .with_context(|| format!("failed to ensure firm {firm_id} exists"))?;
    let company = facade
        .create_company(CreateCompanyRequest {
            firm_id,
            name,
            base_currency: currency,
            tags,
            metadata,
        })
        .await
        .context("failed to create company")?;

    println!(
        "Created company {} ({}) with base currency {}",
        company.name, company.id, company.base_currency
    );

    let bootstrap_summary = bootstrap
        .ensure_bootstrapped(&company)
        .await
        .context("failed to bootstrap ledger state")?;
    if bootstrap_summary.newly_seeded {
        println!(
            "Seeded ledger company {} with {} starter accounts.",
            bootstrap_summary.ledger_company_id, bootstrap_summary.accounts_seeded
        );
    } else {
        println!(
            "Ledger bootstrap already exists for company {} ({} accounts).",
            bootstrap_summary.ledger_company_id, bootstrap_summary.accounts_seeded
        );
    }
    let decision = describe_decision(&bootstrap_summary.policy_preview.outcome.decision);
    let triggers = describe_triggers(&bootstrap_summary.policy_preview.outcome.triggers);
    println!("Policy preview ({decision}): {triggers}");
    let policy_counts = &bootstrap_summary.policy_preview.telemetry;
    println!(
        "Policy telemetry: auto-post {} • needs-approval {} • reject {}",
        policy_counts.policy_auto_post,
        policy_counts.policy_needs_approval,
        policy_counts.policy_reject
    );

    Ok(bootstrap_summary.newly_seeded)
}

async fn archive_company(
    facade: &TenancyFacade,
    firm_id: String,
    company_id: String,
) -> Result<()> {
    let company = facade
        .archive_company(&firm_id, &company_id)
        .await
        .context("failed to archive company")?;
    println!("Archived company {} ({})", company.name, company.id);
    Ok(())
}

async fn reactivate_company(
    facade: &TenancyFacade,
    firm_id: String,
    company_id: String,
) -> Result<()> {
    let company = facade
        .reactivate_company(&firm_id, &company_id)
        .await
        .context("failed to reactivate company")?;
    println!("Reactivated company {} ({})", company.name, company.id);
    Ok(())
}

async fn handle_policy_command(
    _facade: &TenancyFacade,
    bootstrap: &mut AccountingBootstrap,
    command: PolicyCommand,
) -> Result<bool> {
    match command {
        PolicyCommand::Show { company_id } => {
            show_policy(bootstrap, company_id).await?;
            Ok(false)
        }
        PolicyCommand::Set {
            company_id,
            auto_post_limit,
            confidence_floor,
            require_accounts,
            require_vendors,
        } => {
            let changed = set_policy(
                bootstrap,
                &company_id,
                &auto_post_limit,
                confidence_floor,
                require_accounts,
                require_vendors,
            )
            .await?;
            Ok(changed)
        }
    }
}

async fn handle_approvals_command(command: ApprovalsCommand) -> Result<()> {
    match command {
        ApprovalsCommand::Queue { company_id } => {
            let approvals: Arc<dyn codex_approvals::ApprovalsService> =
                Arc::new(InMemoryApprovalsService::new());
            let policy_store: Arc<dyn codex_policy::PolicyStore> =
                Arc::new(InMemoryPolicyStore::new());
            let facade = ControlsFacade::new(policy_store, approvals);
            let view = facade
                .approvals_queue(QueueFilter {
                    company_id: Some(company_id.clone()),
                    status: Some(ApprovalStatus::Pending),
                    assignee: None,
                })
                .await
                .map_err(|err| anyhow!(err))?;
            render_approvals_queue(&company_id, view);
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamFormat {
    Text,
    Json,
}

impl StreamFormat {
    fn is_json(self) -> bool {
        matches!(self, StreamFormat::Json)
    }
}

async fn show_policy(bootstrap: &AccountingBootstrap, company_id: Option<String>) -> Result<()> {
    let mut entries = if let Some(id) = company_id {
        let entry = bootstrap
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("no bootstrap policy found for company {id}"))?;
        vec![entry]
    } else {
        bootstrap.companies().to_vec()
    };

    if entries.is_empty() {
        println!("No policy bootstrap records found.");
        return Ok(());
    }

    entries.sort_by(|a, b| a.tenancy_company_id.cmp(&b.tenancy_company_id));

    let mut rows = Vec::new();
    for entry in entries {
        let preview = preview_policy(
            &entry.tenancy_company_id,
            &entry.base_currency,
            &entry.policy_rules,
        )
        .await?;
        rows.push(build_policy_row(&entry, &preview.outcome));
    }

    render_policy_table(rows);
    Ok(())
}

async fn set_policy(
    bootstrap: &mut AccountingBootstrap,
    company_id: &str,
    auto_post_limit: &str,
    confidence_floor: f32,
    require_accounts: Vec<String>,
    require_vendors: Vec<String>,
) -> Result<bool> {
    if !(0.0..=1.0).contains(&confidence_floor) {
        return Err(anyhow!(
            "confidence floor must be between 0.0 and 1.0; received {confidence_floor}"
        ));
    }
    let limit_minor = parse_amount_minor(auto_post_limit)?;

    let entry = bootstrap
        .policy_entry_mut(company_id)
        .ok_or_else(|| anyhow!("no bootstrap policy found for company {company_id}"))?;

    let mut updated = entry.policy_rules.clone();
    updated.auto_post_enabled = true;
    updated.auto_post_limit_minor = limit_minor;
    updated.confidence_floor = Some(confidence_floor);
    for account in require_accounts {
        if !account.trim().is_empty() {
            updated
                .approval_required_accounts
                .insert(account.trim().to_string());
        }
    }
    for vendor in require_vendors {
        if !vendor.trim().is_empty() {
            updated
                .approval_required_vendors
                .insert(vendor.trim().to_string());
        }
    }

    let changed = updated != entry.policy_rules;
    if changed {
        entry.policy_rules = updated;
    }

    let preview = preview_policy(
        &entry.tenancy_company_id,
        &entry.base_currency,
        &entry.policy_rules,
    )
    .await?;

    println!("Updated policy for {}", entry.tenancy_company_id);
    println!(
        "auto-post limit: {} {} • confidence floor: {}",
        format_minor(entry.policy_rules.auto_post_limit_minor),
        entry.base_currency,
        entry
            .policy_rules
            .confidence_floor
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!(
        "approval accounts: {}",
        join_sorted(&entry.policy_rules.approval_required_accounts)
    );
    println!(
        "approval vendors: {}",
        join_sorted(&entry.policy_rules.approval_required_vendors)
    );
    let decision = describe_decision(&preview.outcome.decision);
    let triggers = describe_triggers(&preview.outcome.triggers);
    println!("Preview ({decision}): {triggers}");

    Ok(changed)
}

fn build_policy_row(entry: &CompanyBootstrap, preview: &EvaluationOutcome) -> Vec<String> {
    vec![
        entry.tenancy_company_id.clone(),
        if entry.policy_rules.auto_post_enabled {
            "enabled".into()
        } else {
            "disabled".into()
        },
        format!(
            "{} {}",
            format_minor(entry.policy_rules.auto_post_limit_minor),
            entry.base_currency
        ),
        entry
            .policy_rules
            .confidence_floor
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "n/a".into()),
        join_sorted(&entry.policy_rules.approval_required_accounts),
        join_sorted(&entry.policy_rules.approval_required_vendors),
        describe_decision(&preview.decision).to_string(),
        describe_triggers(&preview.triggers),
    ]
}

fn render_policy_table(rows: Vec<Vec<String>>) {
    let headers = vec![
        "company_id".to_string(),
        "auto_post".to_string(),
        "limit".to_string(),
        "confidence".to_string(),
        "require_accounts".to_string(),
        "require_vendors".to_string(),
        "preview_decision".to_string(),
        "preview_triggers".to_string(),
    ];
    render_table(headers, rows);
}

fn render_approvals_queue(company_id: &str, view: ApprovalsQueueView) {
    println!(
        "Approvals queue for {company_id} (generated {})",
        view.generated_at.to_rfc3339()
    );
    if view.tasks.is_empty() {
        println!("No pending approvals.");
        return;
    }
    let overdue_ids = view
        .overdue
        .iter()
        .map(|task| task.request.id.clone())
        .collect::<HashSet<_>>();
    let rows = view
        .tasks
        .iter()
        .map(|task| build_approval_row(task, overdue_ids.contains(&task.request.id)))
        .collect::<Vec<_>>();
    let headers = vec![
        "approval_id".to_string(),
        "stage".to_string(),
        "status".to_string(),
        "assignee".to_string(),
        "summary".to_string(),
        "sla_at".to_string(),
        "overdue".to_string(),
    ];
    render_table(headers, rows);
}

fn render_table(headers: Vec<String>, rows: Vec<Vec<String>>) {
    let mut widths = Vec::with_capacity(headers.len());
    for (index, header) in headers.iter().enumerate() {
        let cell_width = rows
            .iter()
            .map(|row| row.get(index).map_or(0, std::string::String::len))
            .max()
            .unwrap_or(0);
        widths.push(max(header.len(), cell_width));
    }

    let mut line = String::new();
    format_table_line(&mut line, &headers, &widths);
    println!("{line}");
    let mut divider = String::new();
    format_table_divider(&mut divider, &widths);
    println!("{divider}");

    for row in rows {
        line.clear();
        format_table_line(&mut line, &row, &widths);
        println!("{line}");
    }
}

fn format_table_line(line: &mut String, cells: &[String], widths: &[usize]) {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            line.push_str(" | ");
        }
        let width = widths.get(index).copied().unwrap_or(cell.len());
        let _ = write!(line, "{cell:<width$}");
    }
}

fn format_table_divider(line: &mut String, widths: &[usize]) {
    for (index, width) in widths.iter().enumerate() {
        if index > 0 {
            line.push_str("-+-");
        }
        line.push_str(&"-".repeat(*width));
    }
}

fn join_sorted(values: &HashSet<String>) -> String {
    if values.is_empty() {
        return "-".into();
    }
    let mut items = values.iter().cloned().collect::<Vec<_>>();
    items.sort();
    items.join(", ")
}

fn parse_amount_minor(raw: &str) -> Result<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("auto-post limit must not be empty"));
    }
    let negative = trimmed.starts_with('-');
    let digits = if negative { &trimmed[1..] } else { trimmed };
    let parts: Vec<&str> = digits.split('.').collect();
    if parts.len() > 2 {
        return Err(anyhow!(
            "auto-post limit {raw} has too many decimal separators"
        ));
    }
    let major_part = parts[0];
    if major_part.is_empty() || !major_part.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(anyhow!("auto-post limit {raw} must start with digits"));
    }
    let major: i64 = major_part
        .parse()
        .context("failed to parse auto-post limit major units")?;
    let fractional = if parts.len() == 2 { parts[1] } else { "" };
    if fractional.chars().any(|ch| !ch.is_ascii_digit()) {
        return Err(anyhow!(
            "auto-post limit fractional portion {fractional} must be numeric"
        ));
    }
    if fractional.len() > 2 {
        return Err(anyhow!(
            "auto-post limit {raw} supports at most two decimal places"
        ));
    }
    let mut fractional_string = fractional.to_string();
    while fractional_string.len() < 2 {
        fractional_string.push('0');
    }
    let minor: i64 = if fractional_string.is_empty() {
        0
    } else {
        fractional_string
            .parse()
            .context("failed to parse auto-post limit fractional units")?
    };
    let mut total = major
        .checked_mul(100)
        .and_then(|value| value.checked_add(minor))
        .ok_or_else(|| anyhow!("auto-post limit {raw} is too large to represent"))?;
    if negative {
        total = -total;
    }
    Ok(total)
}

fn build_approval_row(task: &ApprovalTask, overdue: bool) -> Vec<String> {
    vec![
        task.request.id.clone(),
        format_stage_progress(task),
        format_approval_status(task.status).to_string(),
        task.assigned_to.clone().unwrap_or_else(|| "-".into()),
        task.request.summary.clone(),
        task.request
            .sla_at
            .map(|deadline| deadline.to_rfc3339())
            .unwrap_or_else(|| "-".into()),
        if overdue { "yes".into() } else { "no".into() },
    ]
}

fn format_stage_progress(task: &ApprovalTask) -> String {
    let total = task.stage_decisions.len();
    let completed = task
        .stage_decisions
        .iter()
        .filter(|stage| stage.is_some())
        .count();
    if task.is_finalized() {
        format!("{completed}/{total} complete")
    } else {
        format!(
            "{completed}/{total} complete • stage {}/{}",
            task.current_stage_index + 1,
            total
        )
    }
}

fn format_approval_status(status: ApprovalStatus) -> &'static str {
    match status {
        ApprovalStatus::Pending => "pending",
        ApprovalStatus::Assigned => "assigned",
        ApprovalStatus::Approved => "approved",
        ApprovalStatus::Declined => "declined",
    }
}

struct TenancyStore {
    path: PathBuf,
}

impl TenancyStore {
    fn load() -> Result<(Self, Arc<InMemoryTenancyService>)> {
        let path = Self::default_path()?;
        let snapshot = if path.exists() {
            let data =
                fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
            StoredTenancy::from_slice(&data)?.into_snapshot()?
        } else {
            TenancySnapshot::default()
        };
        let service = Arc::new(InMemoryTenancyService::from_snapshot(snapshot));
        Ok((Self { path }, service))
    }

    async fn persist(&self, service: Arc<InMemoryTenancyService>) -> Result<()> {
        let snapshot = service.export_snapshot().await;
        let stored = StoredTenancy::from_snapshot(snapshot);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let data =
            serde_json::to_vec_pretty(&stored).context("failed to encode tenancy snapshot")?;
        fs::write(&self.path, data)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }

    fn default_path() -> Result<PathBuf> {
        let home = find_codex_home().context("failed to determine Codex home directory")?;
        Ok(Self::storage_path_from_home(home))
    }

    fn storage_path_from_home(mut home: PathBuf) -> PathBuf {
        home.push("accounting");
        home.push("tenancy.json");
        home
    }
}

struct AccountingBootstrap {
    path: PathBuf,
    state: BootstrapState,
}

impl AccountingBootstrap {
    fn load() -> Result<Self> {
        let path = Self::default_path()?;
        let state = if path.exists() {
            let data =
                fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
            let stored: StoredBootstrapState =
                serde_json::from_slice(&data).context("failed to parse bootstrap snapshot")?;
            stored.try_into()?
        } else {
            BootstrapState::default()
        };
        Ok(Self { path, state })
    }

    async fn ensure_bootstrapped(&mut self, company: &Company) -> Result<BootstrapOutcome> {
        if let Some(existing) = self.get(company.id.as_str()).cloned() {
            let preview =
                preview_policy(&company.id, &company.base_currency, &existing.policy_rules).await?;
            return Ok(BootstrapOutcome {
                newly_seeded: false,
                ledger_company_id: existing.ledger_company_id,
                accounts_seeded: existing.accounts_seeded,
                policy_preview: preview,
            });
        }

        let ledger = seed_ledger(company).await?;
        let policy_rules = default_policy_rules(company);
        let preview = preview_policy(&company.id, &company.base_currency, &policy_rules).await?;

        self.state.companies.push(CompanyBootstrap {
            tenancy_company_id: company.id.clone(),
            ledger_company_id: ledger.ledger_company_id.clone(),
            ledger_seeded_at: ledger.seeded_at,
            accounts_seeded: ledger.accounts_seeded,
            policy_rules: policy_rules.clone(),
            base_currency: company.base_currency.clone(),
        });

        Ok(BootstrapOutcome {
            newly_seeded: true,
            ledger_company_id: ledger.ledger_company_id,
            accounts_seeded: ledger.accounts_seeded,
            policy_preview: preview,
        })
    }

    fn get(&self, company_id: &str) -> Option<&CompanyBootstrap> {
        self.state
            .companies
            .iter()
            .find(|entry| entry.tenancy_company_id == company_id)
    }

    fn companies(&self) -> &[CompanyBootstrap] {
        &self.state.companies
    }

    fn policy_entry_mut(&mut self, company_id: &str) -> Option<&mut CompanyBootstrap> {
        self.state
            .companies
            .iter_mut()
            .find(|entry| entry.tenancy_company_id == company_id)
    }

    fn persist(&self) -> Result<()> {
        let stored = StoredBootstrapState::from(&self.state);
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let data =
            serde_json::to_vec_pretty(&stored).context("failed to encode bootstrap snapshot")?;
        fs::write(&self.path, data)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }

    fn default_path() -> Result<PathBuf> {
        let home = find_codex_home().context("failed to determine Codex home directory")?;
        Ok(Self::storage_path_from_home(home))
    }

    fn storage_path_from_home(mut home: PathBuf) -> PathBuf {
        home.push("accounting");
        home.push("bootstrap.json");
        home
    }
}

#[derive(Debug, Default, Clone)]
struct BootstrapState {
    companies: Vec<CompanyBootstrap>,
}

#[derive(Debug, Clone)]
struct CompanyBootstrap {
    tenancy_company_id: String,
    ledger_company_id: String,
    ledger_seeded_at: DateTime<Utc>,
    accounts_seeded: usize,
    policy_rules: PolicyRuleSet,
    base_currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredBootstrapState {
    #[serde(default = "default_bootstrap_version")]
    version: u32,
    #[serde(default)]
    companies: Vec<StoredCompanyBootstrap>,
}

const fn default_bootstrap_version() -> u32 {
    1
}

impl TryFrom<StoredBootstrapState> for BootstrapState {
    type Error = anyhow::Error;

    fn try_from(value: StoredBootstrapState) -> Result<Self, Self::Error> {
        let _version = value.version;
        let companies = value
            .companies
            .into_iter()
            .map(std::convert::TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { companies })
    }
}

impl From<&BootstrapState> for StoredBootstrapState {
    fn from(state: &BootstrapState) -> Self {
        Self {
            version: default_bootstrap_version(),
            companies: state
                .companies
                .iter()
                .map(StoredCompanyBootstrap::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredCompanyBootstrap {
    tenancy_company_id: String,
    ledger_company_id: String,
    ledger_seeded_at: String,
    accounts_seeded: usize,
    policy_rules: PolicyRuleSet,
    #[serde(default)]
    base_currency: Option<String>,
}

impl TryFrom<StoredCompanyBootstrap> for CompanyBootstrap {
    type Error = anyhow::Error;

    fn try_from(value: StoredCompanyBootstrap) -> Result<Self, Self::Error> {
        let seeded_at = DateTime::parse_from_rfc3339(&value.ledger_seeded_at)
            .context("failed to parse bootstrap timestamp")?
            .with_timezone(&Utc);
        Ok(Self {
            tenancy_company_id: value.tenancy_company_id,
            ledger_company_id: value.ledger_company_id,
            ledger_seeded_at: seeded_at,
            accounts_seeded: value.accounts_seeded,
            policy_rules: value.policy_rules,
            base_currency: value.base_currency.unwrap_or_else(|| "USD".to_string()),
        })
    }
}

impl From<&CompanyBootstrap> for StoredCompanyBootstrap {
    fn from(value: &CompanyBootstrap) -> Self {
        Self {
            tenancy_company_id: value.tenancy_company_id.clone(),
            ledger_company_id: value.ledger_company_id.clone(),
            ledger_seeded_at: value.ledger_seeded_at.to_rfc3339(),
            accounts_seeded: value.accounts_seeded,
            policy_rules: value.policy_rules.clone(),
            base_currency: Some(value.base_currency.clone()),
        }
    }
}

struct LedgerSeedSummary {
    ledger_company_id: String,
    accounts_seeded: usize,
    seeded_at: DateTime<Utc>,
}

struct BootstrapOutcome {
    newly_seeded: bool,
    ledger_company_id: String,
    accounts_seeded: usize,
    policy_preview: PolicyPreview,
}

struct PolicyPreview {
    outcome: EvaluationOutcome,
    telemetry: TelemetryCounters,
}

fn default_policy_rules(company: &Company) -> PolicyRuleSet {
    let approval_required_accounts = vec!["2100".to_string(), "6100".to_string()]
        .into_iter()
        .collect::<HashSet<_>>();
    let blocked_vendors = vec!["suspicious-vendor".to_string()]
        .into_iter()
        .collect::<HashSet<_>>();
    PolicyRuleSet {
        auto_post_enabled: false,
        confidence_floor: Some(0.82),
        auto_post_limit_minor: match company.base_currency.as_str() {
            "JPY" => 1_000_000,
            _ => 25_000,
        },
        approval_required_accounts,
        blocked_vendors,
        ..PolicyRuleSet::default()
    }
}

async fn seed_ledger(company: &Company) -> Result<LedgerSeedSummary> {
    let service = Arc::new(InMemoryLedgerService::new());
    let tenant = LedgerTenantContext {
        tenant_id: company.id.clone(),
        user_id: "codex-cli".into(),
        roles: vec![LedgerRole::Admin],
        locale: Some("en-US".into()),
    };
    let ledger_company = service
        .create_company(LedgerCreateCompanyRequest {
            name: company.name.clone(),
            base_currency: LedgerCurrency {
                code: company.base_currency.clone(),
                precision: 2,
            },
            fiscal_calendar: LedgerFiscalCalendar {
                periods_per_year: 12,
                opening_month: 1,
            },
            tenant: tenant.clone(),
        })
        .await
        .map_err(anyhow::Error::from)?;

    let accounts = service
        .seed_chart(SeedChartRequest {
            company_id: ledger_company.id.clone(),
            accounts: default_chart_accounts(),
            tenant,
        })
        .await
        .map_err(anyhow::Error::from)?;

    Ok(LedgerSeedSummary {
        ledger_company_id: ledger_company.id,
        accounts_seeded: accounts.len(),
        seeded_at: Utc::now(),
    })
}

fn default_chart_accounts() -> Vec<ChartAccount> {
    vec![
        ChartAccount {
            code: "1000".to_string(),
            name: "Cash and Cash Equivalents".to_string(),
            account_type: LedgerAccountType::Asset,
            parent_code: None,
            currency_mode: LedgerCurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: false,
        },
        ChartAccount {
            code: "2000".to_string(),
            name: "Accounts Payable".to_string(),
            account_type: LedgerAccountType::Liability,
            parent_code: None,
            currency_mode: LedgerCurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: false,
        },
        ChartAccount {
            code: "4000".to_string(),
            name: "Revenue".to_string(),
            account_type: LedgerAccountType::Revenue,
            parent_code: None,
            currency_mode: LedgerCurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: false,
        },
        ChartAccount {
            code: "6000".to_string(),
            name: "Operating Expenses".to_string(),
            account_type: LedgerAccountType::Expense,
            parent_code: None,
            currency_mode: LedgerCurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: false,
        },
    ]
}

async fn preview_policy(
    company_id: &str,
    currency: &str,
    rules: &PolicyRuleSet,
) -> Result<PolicyPreview> {
    let store: Arc<dyn codex_policy::PolicyStore> = Arc::new(InMemoryPolicyStore::new());
    store
        .put_rule_set(company_id.to_string(), rules.clone())
        .await
        .map_err(|err| anyhow!(err))?;
    // Preview-only telemetry should stay in-memory to avoid persisting synthetic tenant data.
    let telemetry = AccountingTelemetry::new();
    let event_sink: Arc<dyn PolicyEventSink> = Arc::new(telemetry.policy_sink());
    let engine = PolicyEngine::with_components(store, PolicyRuleSet::default(), event_sink);
    let mut proposal =
        PostingProposal::new(company_id.to_string(), rules.auto_post_limit_minor + 1);
    proposal.currency = currency.to_string();
    proposal.confidence = rules.confidence_floor;
    proposal.account_codes = vec!["6000".to_string()];
    let outcome = engine
        .evaluate(
            PolicyContext {
                company_id: company_id.to_string(),
                actor: "codex-cli".to_string(),
            },
            proposal,
        )
        .await
        .map_err(|err| anyhow!(err))?;
    Ok(PolicyPreview {
        outcome,
        telemetry: telemetry.snapshot(),
    })
}

fn policy_summary(rules: &PolicyRuleSet, currency: &str) -> String {
    let limit = format_minor(rules.auto_post_limit_minor);
    if rules.auto_post_enabled {
        let floor = rules
            .confidence_floor
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "n/a".to_string());
        format!("auto-post ≤ {limit} {currency} • confidence ≥ {floor}")
    } else {
        format!("manual approvals • limit {limit} {currency}")
    }
}

fn format_minor(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let major = amount_minor.abs() / 100;
    let minor = amount_minor.abs() % 100;
    format!("{sign}{major}.{minor:02}")
}

fn describe_decision(decision: &PolicyDecision) -> &'static str {
    match decision {
        PolicyDecision::AutoPost => "auto-post",
        PolicyDecision::NeedsApproval => "needs approval",
        PolicyDecision::Reject => "rejected",
    }
}

fn describe_triggers(triggers: &[PolicyTrigger]) -> String {
    if triggers.is_empty() {
        return "no policy blockers".to_string();
    }
    triggers
        .iter()
        .map(describe_trigger)
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_trigger(trigger: &PolicyTrigger) -> String {
    match trigger {
        PolicyTrigger::AutoPostDisabled => "auto-post disabled".to_string(),
        PolicyTrigger::AmountExceedsLimit {
            limit_minor,
            actual_minor,
        } => format!(
            "limit {limit} exceeded by {actual}",
            limit = format_minor(*limit_minor),
            actual = format_minor(*actual_minor)
        ),
        PolicyTrigger::ConfidenceBelowFloor { required, observed } => format!(
            "confidence {observed:.0}% below {required:.0}%",
            observed = observed * 100.0,
            required = required * 100.0
        ),
        PolicyTrigger::ConfidenceMissing { required } => {
            format!("confidence missing (requires ≥ {:.0}%)", required * 100.0)
        }
        PolicyTrigger::VendorRequiresApproval { vendor_id } => {
            format!("vendor {vendor_id} requires approval")
        }
        PolicyTrigger::AccountRequiresApproval { account_code } => {
            format!("account {account_code} requires approval")
        }
        PolicyTrigger::VendorBlocked { vendor_id } => {
            format!("vendor {vendor_id} blocked")
        }
        PolicyTrigger::AccountBlocked { account_code } => {
            format!("account {account_code} blocked")
        }
    }
}

async fn ensure_firm_exists(facade: &TenancyFacade, firm_id: &str) -> Result<Firm> {
    match facade.get_firm(&firm_id.to_string()).await {
        Ok(firm) => Ok(firm),
        Err(TenancyError::NotFound(_)) => match facade
            .create_firm(CreateFirmRequest {
                name: firm_id.to_string(),
                metadata: None,
            })
            .await
        {
            Ok(firm) => Ok(firm),
            Err(TenancyError::Conflict(_)) => facade
                .get_firm(&firm_id.to_string())
                .await
                .map_err(|err| anyhow::anyhow!(err)),
            Err(err) => Err(anyhow::anyhow!(err)),
        },
        Err(err) => Err(anyhow::anyhow!(err)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredTenancy {
    #[serde(default)]
    firms: Vec<StoredFirm>,
    #[serde(default)]
    companies: Vec<StoredCompany>,
    #[serde(default)]
    users: Vec<StoredUser>,
}

impl StoredTenancy {
    fn from_slice(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).context("failed to parse tenancy snapshot")
    }

    fn into_snapshot(self) -> Result<TenancySnapshot> {
        let firms = self
            .firms
            .into_iter()
            .map(StoredFirm::into_firm)
            .collect::<Result<Vec<_>>>()?;
        let companies = self
            .companies
            .into_iter()
            .map(StoredCompany::into_company)
            .collect::<Result<Vec<_>>>()?;
        let users = self
            .users
            .into_iter()
            .map(StoredUser::into_user)
            .collect::<Result<Vec<_>>>()?;
        Ok(TenancySnapshot {
            firms,
            companies,
            users,
        })
    }

    fn from_snapshot(snapshot: TenancySnapshot) -> Self {
        Self {
            firms: snapshot.firms.into_iter().map(StoredFirm::from).collect(),
            companies: snapshot
                .companies
                .into_iter()
                .map(StoredCompany::from)
                .collect(),
            users: snapshot.users.into_iter().map(StoredUser::from).collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredFirm {
    id: String,
    name: String,
    metadata: Option<String>,
    created_at: String,
}

impl StoredFirm {
    fn into_firm(self) -> Result<Firm> {
        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .context("failed to parse firm created_at timestamp")?
            .with_timezone(&Utc);
        Ok(Firm {
            id: self.id,
            name: self.name,
            metadata: self.metadata,
            created_at,
        })
    }

    fn from(firm: Firm) -> Self {
        Self {
            id: firm.id,
            name: firm.name,
            metadata: firm.metadata,
            created_at: firm.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredCompany {
    id: String,
    firm_id: String,
    name: String,
    status: String,
    base_currency: String,
    tags: Vec<String>,
    created_at: String,
    archived_at: Option<String>,
    metadata: Option<String>,
}

impl StoredCompany {
    fn into_company(self) -> Result<Company> {
        let status = match self.status.as_str() {
            "active" => CompanyStatus::Active,
            "archived" => CompanyStatus::Archived,
            other => {
                return Err(anyhow::anyhow!(
                    "unsupported company status {} for {}",
                    other,
                    self.id
                ));
            }
        };

        let created_at = DateTime::parse_from_rfc3339(&self.created_at)
            .context("failed to parse created_at timestamp")?
            .with_timezone(&Utc);
        let archived_at = match self.archived_at {
            Some(value) => Some(
                DateTime::parse_from_rfc3339(&value)
                    .context("failed to parse archived_at timestamp")?
                    .with_timezone(&Utc),
            ),
            None => None,
        };

        Ok(Company {
            id: self.id,
            firm_id: self.firm_id,
            name: self.name,
            status,
            base_currency: self.base_currency,
            tags: self.tags,
            created_at,
            archived_at,
            metadata: self.metadata,
        })
    }
}

// Provide a manual From implementation because derive conflicts with clippy::redundant_closure_for_method_calls.
impl From<Company> for StoredCompany {
    fn from(company: Company) -> Self {
        let status = match company.status {
            CompanyStatus::Active => "active".to_string(),
            CompanyStatus::Archived => "archived".to_string(),
        };

        StoredCompany {
            id: company.id,
            firm_id: company.firm_id,
            name: company.name,
            status,
            base_currency: company.base_currency,
            tags: company.tags,
            created_at: company.created_at.to_rfc3339(),
            archived_at: company.archived_at.map(|value| value.to_rfc3339()),
            metadata: company.metadata,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredUser {
    id: String,
    firm_id: String,
    email: String,
    display_name: String,
    roles: Vec<StoredRoleAssignment>,
    status: String,
    invited_at: String,
    activated_at: Option<String>,
}

impl StoredUser {
    fn into_user(self) -> Result<UserAccount> {
        let invited_at = DateTime::parse_from_rfc3339(&self.invited_at)
            .context("failed to parse invited_at timestamp")?
            .with_timezone(&Utc);
        let activated_at = match self.activated_at {
            Some(value) => Some(
                DateTime::parse_from_rfc3339(&value)
                    .context("failed to parse activated_at timestamp")?
                    .with_timezone(&Utc),
            ),
            None => None,
        };
        let status = parse_user_status(&self.status)?;
        let roles = self
            .roles
            .into_iter()
            .map(StoredRoleAssignment::into_assignment)
            .collect::<Result<Vec<_>>>()?;
        Ok(UserAccount {
            id: self.id,
            firm_id: self.firm_id,
            email: self.email,
            display_name: self.display_name,
            roles,
            status,
            invited_at,
            activated_at,
        })
    }

    fn from(user: UserAccount) -> Self {
        Self {
            id: user.id,
            firm_id: user.firm_id,
            email: user.email,
            display_name: user.display_name,
            roles: user
                .roles
                .into_iter()
                .map(StoredRoleAssignment::from)
                .collect(),
            status: format_user_status(user.status),
            invited_at: user.invited_at.to_rfc3339(),
            activated_at: user.activated_at.map(|ts| ts.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredRoleAssignment {
    role: String,
    scope: StoredRoleScope,
}

impl StoredRoleAssignment {
    fn into_assignment(self) -> Result<RoleAssignment> {
        let role = parse_role(&self.role)?;
        let scope = self.scope.into_scope();
        Ok(RoleAssignment { role, scope })
    }

    fn from(assignment: RoleAssignment) -> Self {
        let role = format_role(assignment.role);
        let scope = StoredRoleScope::from_scope(assignment.scope);
        Self { role, scope }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
enum StoredRoleScope {
    Firm,
    Company { company_id: String },
}

impl StoredRoleScope {
    fn into_scope(self) -> RoleScope {
        match self {
            StoredRoleScope::Firm => RoleScope::FirmWide,
            StoredRoleScope::Company { company_id } => RoleScope::Company(company_id),
        }
    }

    fn from_scope(scope: RoleScope) -> Self {
        match scope {
            RoleScope::FirmWide => StoredRoleScope::Firm,
            RoleScope::Company(company_id) => StoredRoleScope::Company { company_id },
        }
    }
}

fn parse_role(value: &str) -> Result<Role> {
    match value {
        "partner" => Ok(Role::Partner),
        "senior" => Ok(Role::Senior),
        "staff" => Ok(Role::Staff),
        "auditor" => Ok(Role::Auditor),
        other => Err(anyhow::anyhow!("unsupported role {other}")),
    }
}

fn format_role(role: Role) -> String {
    match role {
        Role::Partner => "partner",
        Role::Senior => "senior",
        Role::Staff => "staff",
        Role::Auditor => "auditor",
    }
    .into()
}

fn parse_user_status(value: &str) -> Result<UserStatus> {
    match value {
        "invited" => Ok(UserStatus::Invited),
        "active" => Ok(UserStatus::Active),
        "suspended" => Ok(UserStatus::Suspended),
        "disabled" => Ok(UserStatus::Disabled),
        other => Err(anyhow::anyhow!("unsupported user status {other}")),
    }
}

fn format_user_status(status: UserStatus) -> String {
    match status {
        UserStatus::Invited => "invited",
        UserStatus::Active => "active",
        UserStatus::Suspended => "suspended",
        UserStatus::Disabled => "disabled",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    #[test]
    fn storage_path_from_home_appends_accounting_components() {
        let base = PathBuf::from("/tmp/example/.codex");
        let expected = PathBuf::from("/tmp/example/.codex/accounting/tenancy.json");
        assert_eq!(TenancyStore::storage_path_from_home(base), expected);
    }

    #[test]
    fn storage_path_handles_windows_style_home() {
        let base = PathBuf::from(r"C:\Users\demo\.codex");
        let mut expected = base.clone();
        expected.push("accounting");
        expected.push("tenancy.json");
        let actual = TenancyStore::storage_path_from_home(base);
        assert_eq!(actual, expected);
    }

    #[test]
    fn bootstrap_storage_path_from_home_matches_layout() {
        let base = PathBuf::from("/var/lib/codex");
        let mut expected = base.clone();
        expected.push("accounting");
        expected.push("bootstrap.json");
        let actual = AccountingBootstrap::storage_path_from_home(base);
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn preview_policy_includes_limit_trigger() {
        let rules = PolicyRuleSet {
            auto_post_enabled: false,
            auto_post_limit_minor: 10_000,
            ..PolicyRuleSet::default()
        };
        let preview = preview_policy("company-1", "USD", &rules)
            .await
            .expect("preview should succeed");
        assert_eq!(preview.outcome.decision, PolicyDecision::NeedsApproval);
        assert!(
            preview
                .outcome
                .triggers
                .iter()
                .any(|trigger| matches!(trigger, PolicyTrigger::AutoPostDisabled)),
            "expected autopost disabled trigger"
        );
    }

    #[tokio::test]
    async fn set_policy_updates_rules_in_bootstrap() {
        let mut bootstrap = AccountingBootstrap {
            path: PathBuf::from("/tmp/bootstrap.json"),
            state: BootstrapState {
                companies: vec![CompanyBootstrap {
                    tenancy_company_id: "comp-1".into(),
                    ledger_company_id: "ledger-1".into(),
                    ledger_seeded_at: Utc::now(),
                    accounts_seeded: 4,
                    policy_rules: PolicyRuleSet::default(),
                    base_currency: "USD".into(),
                }],
            },
        };

        let changed = set_policy(
            &mut bootstrap,
            "comp-1",
            "250.50",
            0.85,
            vec!["6100".into()],
            vec!["vendor-1".into()],
        )
        .await
        .expect("set policy should succeed");
        assert!(changed);
        let rules = &bootstrap
            .get("comp-1")
            .expect("company exists")
            .policy_rules;
        assert_eq!(rules.auto_post_limit_minor, 25_050);
        assert_eq!(rules.confidence_floor, Some(0.85));
        assert!(rules.auto_post_enabled);
        assert!(rules.approval_required_accounts.contains("6100"));
        assert!(rules.approval_required_vendors.contains("vendor-1"));

        let no_change = set_policy(
            &mut bootstrap,
            "comp-1",
            "250.50",
            0.85,
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("idempotent policy update");
        assert!(!no_change);
    }

    #[test]
    fn parse_amount_minor_parses_expected_values() {
        assert_eq!(parse_amount_minor("12").expect("major units"), 1_200);
        assert_eq!(parse_amount_minor("12.34").expect("decimal"), 1_234);
        assert_eq!(parse_amount_minor("0.5").expect("fraction"), 50);
        assert!(parse_amount_minor("12.345").is_err());
    }

    #[test]
    fn stored_bootstrap_state_defaults_version() {
        let json = r#"{"companies":[]}"#;
        let stored: StoredBootstrapState =
            serde_json::from_str(json).expect("legacy bootstrap snapshot should deserialize");
        assert_eq!(stored.version, 1);
    }

    #[test]
    fn stored_role_assignment_round_trips() {
        let assignment = RoleAssignment::company(Role::Staff, "company-123".into());
        let stored = StoredRoleAssignment::from(assignment.clone());
        let restored = stored
            .into_assignment()
            .expect("stored assignment should deserialize");
        assert_eq!(restored, assignment);
    }

    #[test]
    fn stored_user_round_trips() {
        let user = UserAccount {
            id: "user-1".into(),
            firm_id: "firm-1".into(),
            email: "user@example.com".into(),
            display_name: "Example User".into(),
            roles: vec![RoleAssignment::firm(Role::Partner)],
            status: UserStatus::Invited,
            invited_at: Utc::now(),
            activated_at: None,
        };

        let stored = StoredUser::from(user.clone());
        let restored = stored.into_user().expect("stored user should deserialize");
        assert_eq!(restored.id, user.id);
        assert_eq!(restored.email, user.email);
        assert_eq!(restored.roles, user.roles);
    }
}
