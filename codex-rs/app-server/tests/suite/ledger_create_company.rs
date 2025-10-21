use std::path::Path;
use std::time::Duration;

use app_test_support::McpProcess;
use app_test_support::to_response;
use codex_app_server_protocol::JSONRPCResponse;
use codex_app_server_protocol::LedgerCreateCompanyParams;
use codex_app_server_protocol::LedgerCreateCompanyResponse;
use codex_app_server_protocol::LedgerCurrency;
use codex_app_server_protocol::LedgerFiscalCalendar;
use codex_app_server_protocol::LedgerLockPeriodResponse;
use codex_app_server_protocol::LedgerPeriodAction;
use codex_app_server_protocol::LedgerPeriodRef;
use codex_app_server_protocol::LedgerPeriodState;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ledger_create_company_provisions_company() {
    let codex_home = TempDir::new().expect("create temp dir");
    create_config_toml(codex_home.path()).expect("write config");

    let mut mcp =
        McpProcess::new_with_env(codex_home.path(), &[("CODEX_LEDGER_IN_MEMORY", Some("1"))])
            .await
            .expect("spawn mcp process");

    timeout(DEFAULT_READ_TIMEOUT, mcp.initialize())
        .await
        .expect("init timeout")
        .expect("init failed");

    let params = LedgerCreateCompanyParams {
        name: "Acme Corp".to_string(),
        base_currency: LedgerCurrency {
            code: "USD".to_string(),
            precision: 2,
        },
        fiscal_calendar: LedgerFiscalCalendar {
            periods_per_year: 12,
            opening_month: 1,
        },
    };

    let request_id = mcp
        .send_ledger_create_company_request(params)
        .await
        .expect("send ledgerCreateCompany");

    let response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(request_id)),
    )
    .await
    .expect("ledgerCreateCompany timeout")
    .expect("ledgerCreateCompany response");

    let LedgerCreateCompanyResponse { company } =
        to_response::<LedgerCreateCompanyResponse>(response)
            .expect("deserialize ledgerCreateCompany response");

    assert_eq!(company.name, "Acme Corp");
    assert_eq!(company.base_currency.code, "USD");
    assert_eq!(company.base_currency.precision, 2);
    assert_eq!(company.fiscal_calendar.periods_per_year, 12);
    assert!(company.id.starts_with("co-"));

    let lock_request_id = mcp
        .send_ledger_lock_period_request(
            company.id.clone(),
            "jnl-gl".to_string(),
            LedgerPeriodRef {
                fiscal_year: 2025,
                period: 1,
            },
            LedgerPeriodAction::SoftClose,
        )
        .await
        .expect("send ledgerLockPeriod");

    let lock_response: JSONRPCResponse = timeout(
        DEFAULT_READ_TIMEOUT,
        mcp.read_stream_until_response_message(RequestId::Integer(lock_request_id)),
    )
    .await
    .expect("ledgerLockPeriod timeout")
    .expect("ledgerLockPeriod response");

    let LedgerLockPeriodResponse { journal } =
        to_response::<LedgerLockPeriodResponse>(lock_response)
            .expect("deserialize ledgerLockPeriod response");

    assert_eq!(journal.company_id, company.id);
    assert_eq!(journal.period_state, LedgerPeriodState::SoftClosed);
}

fn create_config_toml(codex_home: &Path) -> std::io::Result<()> {
    let config_toml = codex_home.join("config.toml");
    std::fs::write(
        config_toml,
        r#"
model = "mock-model"
approval_policy = "never"
sandbox_mode = "danger-full-access"

[model_providers.mock_provider]
name = "Mock provider for ledger test"
base_url = "http://localhost:0/v1"
wire_api = "chat"
request_max_retries = 0
stream_max_retries = 0
"#,
    )
}
