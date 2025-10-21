use anyhow::Result;
use chrono::Duration as ChronoDuration;
use chrono::Utc;
use codex_accounting_api::ReconciliationSummary;
use codex_accounting_api::demo::DemoReconciliationContext;
use codex_accounting_api::demo::seed_demo_reconciliation;
use codex_accounting_api::duplicate_set_labels;
use codex_accounting_api::preview_copy::DUPLICATE_GUIDANCE_PREFIX;
use codex_approvals::ApprovalPriority;
use codex_approvals::ApprovalStatus;
use codex_approvals::ApprovalTask;
use codex_reconcile::CandidateStatus;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;

use crate::preview_wrapping::wrap_bullet_line;
use crate::preview_wrapping::wrap_plain_line;
use crate::preview_wrapping::wrap_prefixed_line;
use crate::preview_wrapping::wrap_subdetail_line;

pub async fn reconciliation_overview_lines() -> Result<Vec<Line<'static>>> {
    let context = seed_demo_reconciliation().await?;
    Ok(build_reconciliation_overview_lines(&context))
}

pub(crate) fn build_reconciliation_overview_lines(
    context: &DemoReconciliationContext,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::with_capacity(128);

    let company_name = context
        .ledger
        .companies
        .first()
        .map(|company| company.name.clone())
        .unwrap_or_else(|| "Demo company".to_string());
    let summary = context
        .facade
        .summary(&context.company_id)
        .unwrap_or(ReconciliationSummary {
            company_id: context.company_id.clone(),
            matched: 0,
            pending: 0,
            last_refreshed_at: None,
        });
    let transactions = context
        .facade
        .list_transactions(&context.company_id)
        .unwrap_or_default();
    let duplicate_sets = duplicate_set_labels(&transactions);
    let candidates = context
        .facade
        .list_candidates(&context.session_id)
        .unwrap_or_default();
    let approvals = &context.approvals_view.tasks;

    lines.extend(wrap_plain_line(vec!["Reconciliation dashboard".bold()]));
    lines.extend(wrap_plain_line(vec![
        "Company ".dim(),
        company_name.cyan().bold(),
        " ".into(),
        format!("({})", context.company_id).dim(),
    ]));

    lines.push(Line::default());
    let ingest_percent = ratio(
        context.ingest_snapshot.deduped_total,
        context.ingest_snapshot.ingested_total,
    );
    let ingest_detail = format!(
        "{} of {} deduped • last feed {}",
        context.ingest_snapshot.deduped_total,
        context.ingest_snapshot.ingested_total,
        relative_since(context.ingest_snapshot.last_ingest_at)
    );
    lines.extend(status_bar_lines(
        "Ingest health",
        ingest_percent,
        ingest_detail,
    ));
    if !duplicate_sets.is_empty() {
        lines.extend(duplicate_reference_lines(&duplicate_sets));
    }

    let total_candidates = summary.matched + summary.pending;
    let coverage_percent = ratio(summary.matched, total_candidates);
    let coverage_detail = format!("{} matched • {} pending", summary.matched, summary.pending);
    lines.extend(status_bar_lines(
        "Reconciliation coverage",
        coverage_percent,
        coverage_detail,
    ));
    if !duplicate_sets.is_empty() {
        lines.extend(duplicate_reference_lines(&duplicate_sets));
    }

    let total_tasks = approvals.len();
    let overdue = context.approvals_view.overdue.len();
    let backlog_percent = if total_tasks == 0 {
        1.0
    } else {
        1.0 - ratio(overdue, total_tasks)
    };
    let backlog_detail = if total_tasks == 0 {
        "queue empty".to_string()
    } else {
        format!("{overdue} overdue • {total_tasks} open")
    };
    lines.extend(status_bar_lines(
        "Approvals backlog",
        backlog_percent,
        backlog_detail,
    ));

    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec!["Bank transactions".bold()]));
    for tx in transactions.iter().take(4) {
        let amount_text = format_currency(tx.amount_minor);
        let amount_span = if tx.amount_minor >= 0 {
            amount_text.green()
        } else {
            amount_text.red()
        };
        let content = vec![
            tx.posted_date.to_string().dim(),
            " ".into(),
            tx.description.clone().into(),
            " ".into(),
            amount_span,
        ];
        lines.extend(wrap_bullet_line(content));
        let mut metadata: Vec<Span<'static>> = Vec::new();
        metadata.push("account ".dim());
        metadata.push(tx.account_id.clone().into());
        if let Some(reference) = &tx.source_reference {
            metadata.push(" • ".dim());
            metadata.push(format!("ref {reference}").dim());
        }
        if tx.duplicate_metadata.total_occurrences > 1 {
            metadata.push(" • ".dim());
            metadata.push(
                format!(
                    "{} dup trimmed",
                    tx.duplicate_metadata.total_occurrences.saturating_sub(1)
                )
                .dim(),
            );
        }
        if !metadata.is_empty() {
            lines.extend(wrap_subdetail_line(metadata));
        }
    }
    if transactions.len() > 4 {
        lines.extend(wrap_plain_line(vec![
            "  … additional transactions trimmed".dim(),
        ]));
    }

    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec!["Match candidates".bold()]));
    for candidate in candidates.iter().take(4) {
        let status_span = candidate_status_span(candidate.status);
        let score_pct = format!("{:.0}%", (candidate.score * 100.0).clamp(0.0, 100.0)).dim();
        let content = vec![
            candidate.transaction_id.clone().bold(),
            " ".into(),
            "⇔".dim(),
            " ".into(),
            candidate.journal_entry_id.clone().into(),
            " ".into(),
            status_span,
            " ".into(),
            score_pct,
        ];
        lines.extend(wrap_bullet_line(content));
        let mut detail: Vec<Span<'static>> = Vec::new();
        if let Some(group) = &candidate.group_id {
            detail.push("group ".dim());
            detail.push(group.clone().into());
        }
        if let Some(reason) = &candidate.write_off_reason {
            if !detail.is_empty() {
                detail.push(" • ".dim());
            }
            detail.push("write-off ".dim());
            detail.push(reason.clone().magenta());
        }
        if !detail.is_empty() {
            lines.extend(wrap_subdetail_line(detail));
        }
    }
    if candidates.len() > 4 {
        lines.extend(wrap_plain_line(vec![
            "  … additional candidates trimmed".dim(),
        ]));
    }

    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec!["SLA indicators".bold()]));
    lines.extend(wrap_bullet_line(vec![
        "Overdue approvals ".dim(),
        overdue.to_string().red(),
        " of ".into(),
        total_tasks.to_string().into(),
    ]));
    let next_deadline = approvals
        .iter()
        .filter_map(|task| task.request.sla_at)
        .min();
    if let Some(deadline) = next_deadline {
        lines.extend(wrap_bullet_line(vec![
            "Next SLA ".dim(),
            relative_until(deadline).bold(),
        ]));
    } else {
        lines.extend(wrap_bullet_line(vec!["No SLA targets scheduled".dim()]));
    }

    if !approvals.is_empty() {
        lines.push(Line::default());
        lines.extend(wrap_plain_line(vec!["Approvals backlog details".bold()]));
        for task in approvals.iter().take(3) {
            lines.extend(approval_task_lines(task));
        }
        if approvals.len() > 3 {
            lines.extend(wrap_plain_line(vec![
                "  … additional approvals trimmed".dim(),
            ]));
        }
    }

    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec![
        "Snapshot generated ".dim(),
        relative_since(context.approvals_view.generated_at).dim(),
    ]));
    let counters = context.telemetry.snapshot();
    lines.extend(wrap_plain_line(vec![
        "Telemetry ".dim(),
        format!(
            "tx {} • candidates {} • write-offs {}",
            counters.reconciliation_transactions,
            counters.reconciliation_candidates,
            counters.reconciliation_write_offs
        )
        .dim(),
    ]));
    lines.extend(wrap_plain_line(vec![
        "Period locks ".dim(),
        format!(
            "{} total • close {} • soft {} • reopen {}",
            counters.period_lock_events,
            counters.period_lock_close,
            counters.period_lock_soft_close,
            counters.period_lock_reopen_soft + counters.period_lock_reopen_full
        )
        .dim(),
    ]));
    lines.extend(wrap_plain_line(vec![
        "Policy decisions ".dim(),
        format!(
            "auto-post {} • needs-approval {} • reject {}",
            counters.policy_auto_post, counters.policy_needs_approval, counters.policy_reject
        )
        .dim(),
    ]));
    lines.extend(wrap_plain_line(vec![
        "Approvals snapshot ".dim(),
        format!(
            "{} open / {} overdue",
            counters.approvals_total, counters.approvals_overdue
        )
        .dim(),
    ]));
    let telemetry_file = context
        .telemetry
        .store_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "in-memory (set CODEX_HOME to persist)".to_string());
    lines.extend(wrap_plain_line(vec![
        "Telemetry file ".dim(),
        telemetry_file.dim(),
    ]));

    lines
}

pub fn error_lines(message: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.extend(wrap_plain_line(vec!["Reconciliation dashboard".bold()]));
    lines.extend(wrap_plain_line(vec![
        "Unable to load demo reconciliation data:".red(),
    ]));
    lines.extend(wrap_plain_line(vec![message.to_string().red()]));
    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec![
        "Try running `codex ledger reconciliation summary` for details.".dim(),
    ]));
    lines
}

fn duplicate_reference_lines(duplicate_sets: &[String]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for key in duplicate_sets {
        lines.extend(wrap_subdetail_line(vec![
            DUPLICATE_GUIDANCE_PREFIX.dim(),
            key.clone().dim(),
        ]));
    }
    lines
}

fn status_bar_lines(label: &str, percent: f32, detail: String) -> Vec<Line<'static>> {
    let bar = progress_bar(percent);
    let colored_bar = if percent >= 0.85 {
        bar.green()
    } else if percent >= 0.6 {
        bar.cyan()
    } else {
        bar.red()
    };
    let percentage = format!("{:.0}%", (percent * 100.0).clamp(0.0, 100.0)).dim();
    let content = vec![
        colored_bar,
        " ".into(),
        percentage,
        " ".into(),
        detail.dim(),
    ];
    let initial_prefix = Line::from(vec![label.to_string().bold(), " ".into()]);
    // Dynamic indent keeps wrapped detail text aligned with the status label.
    let subsequent_spaces = " ".repeat(label.chars().count() + 1);
    let subsequent_prefix = Line::from(vec![Span::from(subsequent_spaces)]);
    wrap_prefixed_line(content, initial_prefix, subsequent_prefix)
}

fn approval_task_lines(task: &ApprovalTask) -> Vec<Line<'static>> {
    let status_span = approval_status_span(task.status);
    let amount_span = Span::from(format_currency(task.request.amount_minor)).cyan();
    let mut content = vec![
        task.request.summary.clone().bold(),
        " ".into(),
        status_span,
        " ".into(),
        amount_span,
        " ".into(),
        task.request.currency.clone().dim(),
    ];
    if let Some(priority) = approval_priority_span(task.request.priority) {
        content.push(" ".into());
        content.push(priority);
    }
    let mut lines = wrap_bullet_line(content);
    let total_stages = task.request.stages.len().max(1);
    let mut detail = vec![
        "stage ".dim(),
        format!("{}/{}", task.current_stage_index + 1, total_stages).into(),
    ];
    if let Some(sla) = task.request.sla_at {
        detail.push(" • ".dim());
        detail.push("SLA ".dim());
        detail.push(relative_until(sla).dim());
    }
    detail.push(" • ".dim());
    detail.push("submitted ".dim());
    detail.push(relative_since(task.request.submitted_at).dim());
    if let Some(assignee) = &task.assigned_to {
        detail.push(" • ".dim());
        detail.push(format!("assignee {assignee}").dim());
    }
    if let Some(metadata) = &task.request.metadata {
        detail.push(" • ".dim());
        detail.push(metadata.clone().dim());
    }
    lines.extend(wrap_subdetail_line(detail));
    lines
}

fn approval_status_span(status: ApprovalStatus) -> Span<'static> {
    match status {
        ApprovalStatus::Pending => "pending".cyan(),
        ApprovalStatus::Assigned => "assigned".cyan(),
        ApprovalStatus::Approved => "approved".green(),
        ApprovalStatus::Declined => "declined".red(),
    }
}

fn approval_priority_span(priority: ApprovalPriority) -> Option<Span<'static>> {
    match priority {
        ApprovalPriority::High => Some("HIGH".magenta().bold()),
        ApprovalPriority::Low => Some("LOW".dim()),
        ApprovalPriority::Normal => None,
    }
}

fn progress_bar(percent: f32) -> String {
    let total = 20;
    let filled = (percent.clamp(0.0, 1.0) * total as f32).round() as usize;
    let clamped = filled.min(total);
    let empty = total.saturating_sub(clamped);
    format!("[{}{}]", "#".repeat(clamped), ".".repeat(empty))
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn format_currency(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let value = amount_minor.abs();
    let units = value / 100;
    let cents = value % 100;
    format!("{sign}${units}.{cents:02}")
}

fn candidate_status_span(status: CandidateStatus) -> Span<'static> {
    match status {
        CandidateStatus::Pending => "pending".cyan(),
        CandidateStatus::PartiallyAccepted => "partial".cyan(),
        CandidateStatus::Accepted => "accepted".green(),
        CandidateStatus::Rejected => "rejected".red(),
        CandidateStatus::WrittenOff => "written-off".magenta(),
    }
}

fn relative_since(instant: chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    if instant >= now {
        "just now".to_string()
    } else {
        let delta = now - instant;
        format!("{} ago", human_duration(delta))
    }
}

fn relative_until(instant: chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    if instant >= now {
        let delta = instant - now;
        format!("due in {}", human_duration(delta))
    } else {
        let delta = now - instant;
        format!("overdue by {}", human_duration(delta))
    }
}

fn human_duration(delta: ChronoDuration) -> String {
    let total_seconds = delta.num_seconds().abs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours > 0 {
        if minutes > 0 {
            format!("{hours}h {minutes:02}m")
        } else {
            format!("{hours}h")
        }
    } else if minutes > 0 {
        format!("{minutes}m")
    } else {
        format!("{total_seconds}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use regex_lite::Regex;

    #[tokio::test]
    async fn overview_lines_render() {
        let lines = reconciliation_overview_lines().await.expect("lines");
        assert!(!lines.is_empty());
    }

    #[test]
    fn progress_bar_bounds() {
        assert_eq!(progress_bar(-1.0), "[....................]");
        assert_eq!(progress_bar(0.5), "[##########..........]");
        assert_eq!(progress_bar(1.5), "[####################]");
    }

    #[tokio::test]
    async fn overview_lines_snapshot() {
        let lines = reconciliation_overview_lines().await.expect("lines");
        let rendered = lines
            .iter()
            .map(line_contents)
            .collect::<Vec<_>>()
            .join("\n");
        let sanitized = sanitize_for_snapshot(rendered);
        assert_snapshot!("reconciliation_dashboard_preview", sanitized);
    }

    #[tokio::test]
    async fn duplicate_guidance_printed_for_each_status_bar() {
        let lines = reconciliation_overview_lines().await.expect("lines");
        let context = seed_demo_reconciliation().await.expect("context");
        let transactions = context
            .facade
            .list_transactions(&context.company_id)
            .unwrap_or_default();
        let expected = duplicate_set_labels(&transactions);
        let duplicates_rendered = lines
            .iter()
            .filter(|line| line_contents(line).contains(DUPLICATE_GUIDANCE_PREFIX))
            .count();
        assert_eq!(duplicates_rendered, expected.len() * 2);
    }

    #[cfg(feature = "reconciliation-dup-snapshots")]
    #[tokio::test]
    async fn overview_lines_snapshot_extended_duplicates() {
        let mut context = seed_demo_reconciliation().await.expect("context");
        let mut transactions = context
            .facade
            .list_transactions(&context.company_id)
            .unwrap_or_default();
        let mut with_reference = transactions.get(1).cloned().expect("sample transaction");
        with_reference.transaction_id = "txn-extra-ref-keeper".to_string();
        with_reference.description = "Extended duplicate with reference".to_string();
        with_reference.source_reference = Some("REF-EXT-001".to_string());
        with_reference.duplicate_metadata.group_key = Some("grp-ext-ref".to_string());
        with_reference.duplicate_metadata.total_occurrences = 4;
        with_reference.duplicate_metadata.discarded_ids = vec![
            "txn-ext-ref-1".into(),
            "txn-ext-ref-2".into(),
            "txn-ext-ref-3".into(),
        ];
        let mut group_only = transactions.get(2).cloned().expect("secondary transaction");
        group_only.transaction_id = "txn-extra-group-keeper".to_string();
        group_only.description = "Duplicate without reference".to_string();
        group_only.source_reference = None;
        group_only.duplicate_metadata.group_key = Some("grp-ext-group".to_string());
        group_only.duplicate_metadata.total_occurrences = 2;
        group_only.duplicate_metadata.discarded_ids = vec!["txn-ext-group-shadow".into()];
        let mut fallback = transactions.get(3).cloned().expect("tertiary transaction");
        fallback.transaction_id = "txn-fallback-keeper".to_string();
        fallback.description = "Duplicate falling back to transaction id".to_string();
        fallback.source_reference = None;
        fallback.duplicate_metadata.group_key = None;
        fallback.duplicate_metadata.total_occurrences = 3;
        fallback.duplicate_metadata.discarded_ids = vec![
            "txn-fallback-shadow-1".into(),
            "txn-fallback-shadow-2".into(),
        ];
        transactions.extend([with_reference, group_only, fallback]);
        context
            .transactions_source
            .insert(&context.company_id, transactions.clone());
        let duplicates_dropped = transactions
            .iter()
            .map(|tx| tx.duplicate_metadata.total_occurrences.saturating_sub(1))
            .sum::<usize>();
        context.ingest_snapshot.deduped_total = transactions.len();
        context.ingest_snapshot.duplicates_dropped = duplicates_dropped;
        context.ingest_snapshot.ingested_total =
            context.ingest_snapshot.deduped_total + duplicates_dropped;
        let lines = build_reconciliation_overview_lines(&context);
        let rendered = lines
            .iter()
            .map(line_contents)
            .collect::<Vec<_>>()
            .join("\n");
        let sanitized = sanitize_for_snapshot(rendered);
        assert_snapshot!("reconciliation_dashboard_preview_extended_dups", sanitized);
    }

    #[cfg(feature = "reconciliation-dup-snapshots")]
    #[tokio::test]
    async fn overview_lines_scales_with_large_transaction_volume() {
        let mut context = seed_demo_reconciliation().await.expect("context");
        let base_transactions = context
            .facade
            .list_transactions(&context.company_id)
            .unwrap_or_default();
        assert!(!base_transactions.is_empty());
        let mut transactions = Vec::with_capacity(1200);
        for idx in 0..1200 {
            let mut tx = base_transactions[idx % base_transactions.len()].clone();
            tx.transaction_id = format!("txn-load-{idx:04}");
            tx.description = format!("Load test transaction {idx:04}");
            if idx % 9 == 0 {
                tx.source_reference = Some(format!("REF-LOAD-{idx:04}"));
                tx.duplicate_metadata.group_key = Some(format!("grp-load-{idx:04}"));
                tx.duplicate_metadata.total_occurrences = 2;
                tx.duplicate_metadata.discarded_ids = vec![format!("txn-load-shadow-{idx:04}")];
            } else {
                tx.source_reference = None;
                tx.duplicate_metadata.group_key = None;
                tx.duplicate_metadata.total_occurrences = 1;
                tx.duplicate_metadata.discarded_ids.clear();
            }
            transactions.push(tx);
        }
        context
            .transactions_source
            .insert(&context.company_id, transactions.clone());
        let duplicates_dropped = transactions
            .iter()
            .map(|tx| tx.duplicate_metadata.total_occurrences.saturating_sub(1))
            .sum::<usize>();
        context.ingest_snapshot.deduped_total = transactions.len();
        context.ingest_snapshot.duplicates_dropped = duplicates_dropped;
        context.ingest_snapshot.ingested_total =
            context.ingest_snapshot.deduped_total + duplicates_dropped;
        let lines = build_reconciliation_overview_lines(&context);
        assert!(!lines.is_empty());
        let duplicate_lines = lines
            .iter()
            .filter(|line| line_contents(line).contains(DUPLICATE_GUIDANCE_PREFIX))
            .count();
        // Every ninth transaction is a duplicate; ensure both status bars surface guidance.
        let expected_duplicate_sets = transactions
            .iter()
            .filter(|tx| tx.duplicate_metadata.total_occurrences > 1)
            .count();
        assert_eq!(duplicate_lines, expected_duplicate_sets * 2);
    }

    fn line_contents(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    fn sanitize_for_snapshot(mut text: String) -> String {
        let replacements = [
            (r"[0-9]{4}-[0-9]{2}-[0-9]{2}", "<DATE>"),
            (r"due in [0-9]+h(?: [0-9]{2}m)?", "due in <DURATION>"),
            (
                r"overdue by [0-9]+h(?: [0-9]{2}m)?",
                "overdue by <DURATION>",
            ),
            (r"[0-9]+h(?: [0-9]{2}m)? ago", "<AGO>"),
            (r"[0-9]+m ago", "<AGO>"),
            (r"[0-9]+s ago", "<AGO>"),
        ];
        for (pattern, replacement) in replacements {
            let regex = Regex::new(pattern).expect("valid regex");
            text = regex.replace_all(&text, replacement).into_owned();
        }
        let duplicates_pattern = format!(r"\b{DUPLICATE_GUIDANCE_PREFIX}(?P<id>\S+)");
        let duplicates_regex = Regex::new(&duplicates_pattern).expect("valid regex");
        let duplicates_replacement = format!("{DUPLICATE_GUIDANCE_PREFIX}<ID>");
        text = duplicates_regex
            .replace_all(&text, duplicates_replacement.as_str())
            .into_owned();
        text
    }
}
