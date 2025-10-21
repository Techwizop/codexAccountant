use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use codex_accounting_api::LedgerFacade;
use codex_accounting_api::demo::DemoLedgerData;
use codex_accounting_api::demo::DemoLedgerEntry;
use codex_accounting_api::demo::seed_demo_ledger;
use codex_app_server_protocol::LedgerAccount;
use codex_app_server_protocol::LedgerCompany;
use codex_ledger::InMemoryLedgerService;
use codex_ledger::LedgerService;
use ratatui::style::Stylize;
use ratatui::text::Line;

use crate::preview_wrapping::wrap_bullet_line;
use crate::preview_wrapping::wrap_plain_line;
pub async fn ledger_summary_lines() -> Result<Vec<Line<'static>>> {
    let service: Arc<dyn LedgerService> = Arc::new(InMemoryLedgerService::new());
    let facade = LedgerFacade::new(service);
    let data = seed_demo_ledger(&facade)
        .await
        .context("failed to seed demo ledger")?;
    Ok(summary_lines_from_data(&data))
}

pub fn summary_lines_from_data(data: &DemoLedgerData) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.extend(wrap_plain_line(vec!["Ledger snapshot".bold()]));
    lines.extend(wrap_plain_line(vec![
        "Prototype summary for demo data.".dim(),
    ]));
    lines.push(Line::default());

    if data.companies.is_empty() {
        lines.extend(wrap_plain_line(vec![
            "No companies found in the demo ledger.".into(),
        ]));
        return lines;
    }

    for (idx, company) in data.companies.iter().enumerate() {
        if idx > 0 {
            lines.push(Line::default());
        }
        lines.extend(company_header(company));
        let accounts = accounts_for_company(&data.accounts, &company.id);
        if accounts.is_empty() {
            lines.extend(wrap_plain_line(vec!["  No accounts configured.".dim()]));
        } else {
            lines.extend(wrap_plain_line(vec!["  Accounts".bold()]));
            for account in accounts {
                lines.extend(wrap_bullet_line(vec![
                    account.code.clone().bold(),
                    " ".into(),
                    account.name.clone().into(),
                    " ".into(),
                    format!("({:?})", account.account_type).dim(),
                ]));
            }
        }

        let entries = entries_for_company(&data.entries, &company.id);
        if entries.is_empty() {
            lines.extend(wrap_plain_line(vec!["  No journal entries posted.".dim()]));
        } else {
            lines.extend(wrap_plain_line(vec!["  Recent journal entries".bold()]));
            for entry in entries {
                lines.extend(wrap_bullet_line(vec![
                    entry.entry.id.clone().bold(),
                    " ".into(),
                    format!("({:?})", entry.entry.status).magenta(),
                    " ".into(),
                    format!("{} lines", entry.entry.lines.len()).dim(),
                ]));
            }
        }
    }

    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec![
        "Hint: Run `codex ledger demo` for CLI seeding.".dim(),
    ]));

    lines
}

pub fn error_lines(message: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.extend(wrap_plain_line(vec!["Ledger snapshot".bold()]));
    lines.extend(wrap_plain_line(vec!["Unable to load demo data:".red()]));
    lines.extend(wrap_plain_line(vec![message.to_string().red()]));
    lines.push(Line::default());
    lines.extend(wrap_plain_line(vec![
        "Try `codex ledger demo` from the CLI for more details.".dim(),
    ]));
    lines
}

fn company_header(company: &LedgerCompany) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.extend(wrap_plain_line(vec![
        "Company ".dim(),
        company.name.clone().cyan().bold(),
        " ".into(),
        format!("({})", company.id).dim(),
    ]));
    lines.extend(wrap_plain_line(vec![
        "  Base currency ".dim(),
        company.base_currency.code.clone().green(),
        " ".into(),
        format!("(precision {} digits)", company.base_currency.precision).dim(),
    ]));
    lines.extend(wrap_plain_line(vec![
        "  Fiscal calendar ".dim(),
        format!(
            "{} periods / opens in month {}",
            company.fiscal_calendar.periods_per_year, company.fiscal_calendar.opening_month
        )
        .into(),
    ]));
    lines
}

fn accounts_for_company<'a>(
    accounts: &'a [LedgerAccount],
    company_id: &str,
) -> Vec<&'a LedgerAccount> {
    accounts
        .iter()
        .filter(|account| account.company_id == company_id)
        .collect()
}

fn entries_for_company<'a>(
    entries: &'a [DemoLedgerEntry],
    company_id: &str,
) -> Vec<&'a DemoLedgerEntry> {
    entries
        .iter()
        .filter(|entry| entry.company_id == company_id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_consts::PREVIEW_WRAP_WIDTH;
    use codex_app_server_protocol::LedgerAccountType;
    use codex_app_server_protocol::LedgerCurrencyMode;
    use codex_app_server_protocol::LedgerEntryOrigin;
    use codex_app_server_protocol::LedgerJournalEntry;
    use codex_app_server_protocol::LedgerJournalLine;
    use codex_app_server_protocol::LedgerPostingSide;
    use codex_app_server_protocol::LedgerReconciliationStatus;
    use pretty_assertions::assert_eq;
    use unicode_width::UnicodeWidthStr;

    fn demo_company() -> LedgerCompany {
        LedgerCompany {
            id: "co-1".to_string(),
            name: "Demo Co".to_string(),
            base_currency: codex_app_server_protocol::LedgerCurrency {
                code: "USD".to_string(),
                precision: 2,
            },
            fiscal_calendar: codex_app_server_protocol::LedgerFiscalCalendar {
                periods_per_year: 12,
                opening_month: 1,
            },
            metadata: None,
        }
    }

    fn demo_account(
        id: &str,
        code: &str,
        name: &str,
        account_type: LedgerAccountType,
    ) -> LedgerAccount {
        LedgerAccount {
            id: id.to_string(),
            company_id: "co-1".to_string(),
            code: code.to_string(),
            name: name.to_string(),
            account_type,
            parent_account_id: None,
            currency_mode: LedgerCurrencyMode::FunctionalOnly,
            tax_code: None,
            is_summary: false,
            is_active: true,
        }
    }

    fn demo_entry() -> DemoLedgerEntry {
        DemoLedgerEntry {
            company_id: "co-1".to_string(),
            entry: LedgerJournalEntry {
                id: "je-1".to_string(),
                journal_id: "jnl-gl".to_string(),
                status: codex_app_server_protocol::LedgerEntryStatus::Posted,
                reconciliation_status: LedgerReconciliationStatus::Reconciled {
                    session_id: "session-1".into(),
                },
                lines: vec![
                    LedgerJournalLine {
                        id: "ln-1".to_string(),
                        account_id: "cash".to_string(),
                        side: LedgerPostingSide::Debit,
                        amount_minor: 10_000,
                        currency: codex_app_server_protocol::LedgerCurrency {
                            code: "USD".to_string(),
                            precision: 2,
                        },
                        functional_amount_minor: 10_000,
                        functional_currency: codex_app_server_protocol::LedgerCurrency {
                            code: "USD".to_string(),
                            precision: 2,
                        },
                        exchange_rate: None,
                        tax_code: None,
                        memo: Some("Sale".to_string()),
                    },
                    LedgerJournalLine {
                        id: "ln-2".to_string(),
                        account_id: "revenue".to_string(),
                        side: LedgerPostingSide::Credit,
                        amount_minor: 10_000,
                        currency: codex_app_server_protocol::LedgerCurrency {
                            code: "USD".to_string(),
                            precision: 2,
                        },
                        functional_amount_minor: 10_000,
                        functional_currency: codex_app_server_protocol::LedgerCurrency {
                            code: "USD".to_string(),
                            precision: 2,
                        },
                        exchange_rate: None,
                        tax_code: None,
                        memo: Some("Sale".to_string()),
                    },
                ],
                origin: LedgerEntryOrigin::Manual,
                memo: Some("Demo entry".to_string()),
                reverses_entry_id: None,
                reversed_by_entry_id: None,
            },
        }
    }

    #[test]
    fn snapshot_summary_lines() {
        let data = DemoLedgerData {
            companies: vec![demo_company()],
            accounts: vec![
                demo_account("cash", "1000", "Cash", LedgerAccountType::Asset),
                demo_account("revenue", "4000", "Revenue", LedgerAccountType::Revenue),
            ],
            entries: vec![demo_entry()],
        };

        let lines = summary_lines_from_data(&data);
        let contents: Vec<String> = lines.iter().map(line_contents).collect();
        assert_eq!(
            contents,
            vec![
                "Ledger snapshot".to_string(),
                "Prototype summary for demo data.".to_string(),
                "".to_string(),
                "Company Demo Co (co-1)".to_string(),
                "  Base currency USD (precision 2 digits)".to_string(),
                "  Fiscal calendar 12 periods / opens in month 1".to_string(),
                "  Accounts".to_string(),
                "  • 1000 Cash (Asset)".to_string(),
                "  • 4000 Revenue (Revenue)".to_string(),
                "  Recent journal entries".to_string(),
                "  • je-1 (Posted) 2 lines".to_string(),
                "".to_string(),
                "Hint: Run `codex ledger demo` for CLI seeding.".to_string(),
            ]
        );
    }

    #[test]
    fn snapshot_error_lines() {
        let lines = error_lines("boom");
        let contents: Vec<String> = lines.iter().map(line_contents).collect();
        assert_eq!(
            contents,
            vec![
                "Ledger snapshot".to_string(),
                "Unable to load demo data:".to_string(),
                "boom".to_string(),
                "".to_string(),
                "Try `codex ledger demo` from the CLI for more details.".to_string(),
            ]
        );
    }

    #[test]
    fn error_lines_wrap_long_message_to_width() {
        let message = "Failed to connect to ledger reconciliation preview service; ensure CLI telemetry path is configured and the seatbelt sandbox is disabled for streaming mode.";
        let lines = error_lines(message);
        let contents: Vec<String> = lines.iter().map(line_contents).collect();
        assert!(
            contents
                .iter()
                .any(|line| line.contains("Failed to connect")),
            "long message missing from rendered lines: {contents:?}"
        );
        let widths: Vec<usize> = contents
            .iter()
            .filter(|line| !line.is_empty())
            .map(|line| UnicodeWidthStr::width(line.as_str()))
            .collect();
        assert!(
            widths.iter().all(|width| *width <= PREVIEW_WRAP_WIDTH),
            "line widths {widths:?} exceed wrap width {PREVIEW_WRAP_WIDTH}"
        );
    }

    fn line_contents(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }
}
