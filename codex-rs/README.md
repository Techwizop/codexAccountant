# Codex CLI (Rust Implementation)

We provide Codex CLI as a standalone, native executable to ensure a zero-dependency install.

## Prerequisites

Before running the development workflows in this repository, confirm the following tools are installed and on your `PATH`:

- Latest Rust toolchain via [`rustup`](https://rustup.rs/)
- [`just`](https://github.com/casey/just) task runner
- [`rg`](https://github.com/BurntSushi/ripgrep) (`ripgrep`) for fast search
- [`cargo-insta`](https://insta.rs/docs/quickstart/) for snapshot management

Typical install commands (feel free to swap in your preferred package manager):

```bash
rustup update
cargo install just cargo-insta
cargo install ripgrep        # or: brew install ripgrep / apt install ripgrep
```

## Installing Codex

Today, the easiest way to install Codex is via `npm`:

```shell
npm i -g @openai/codex
codex
```

You can also install via Homebrew (`brew install codex`) or download a platform-specific release directly from our [GitHub Releases](https://github.com/openai/codex/releases).

## What's new in the Rust CLI

The Rust implementation is now the maintained Codex CLI and serves as the default experience. It includes a number of features that the legacy TypeScript CLI never supported.

### Config

Codex supports a rich set of configuration options. Note that the Rust CLI uses `config.toml` instead of `config.json`. See [`docs/config.md`](../docs/config.md) for details.

### Model Context Protocol Support

#### MCP client

Codex CLI functions as an MCP client that allows the Codex CLI and IDE extension to connect to MCP servers on startup. See the [`configuration documentation`](../docs/config.md#mcp_servers) for details.

#### MCP server (experimental)

Codex can be launched as an MCP _server_ by running `codex mcp-server`. This allows _other_ MCP clients to use Codex as a tool for another agent.

Use the [`@modelcontextprotocol/inspector`](https://github.com/modelcontextprotocol/inspector) to try it out:

```shell
npx @modelcontextprotocol/inspector codex mcp-server
```

Use `codex mcp` to add/list/get/remove MCP server launchers defined in `config.toml`, and `codex mcp-server` to run the MCP server directly.

### Notifications

You can enable notifications by configuring a script that is run whenever the agent finishes a turn. The [notify documentation](../docs/config.md#notify) includes a detailed example that explains how to get desktop notifications via [terminal-notifier](https://github.com/julienXX/terminal-notifier) on macOS.

### `codex exec` to run Codex programmatically/non-interactively

To run Codex non-interactively, run `codex exec PROMPT` (you can also pass the prompt via `stdin`) and Codex will work on your task until it decides that it is done and exits. Output is printed to the terminal directly. You can set the `RUST_LOG` environment variable to see more about what's going on.

### Use `@` for file search

Typing `@` triggers a fuzzy-filename search over the workspace root. Use up/down to select among the results and Tab or Enter to replace the `@` with the selected path. You can use Esc to cancel the search.

### Accounting preview commands

Codex ships an experimental ledger sandbox that demonstrates the accounting workflow:

- `codex ledger demo` seeds an in-memory ledger, configures starter accounts, and posts the first journal entry.
- Read-only helpers such as `codex ledger companies`, `codex ledger accounts`, and `codex ledger entries` expose the seeded data for quick inspection.

See the [Accounting CLI reference](../docs/accounting/cli.md) for JSON schemas, sanitized examples, and streaming samples that match the commands below.

#### `codex ledger list-locks`
- Preview lock history, approval references, and telemetry counters for the demo ledger.

```shell
codex ledger list-locks
codex ledger list-locks --format json
```

`Telemetry file:` lines indicate where counters persist (set `CODEX_HOME` to capture them).

#### `codex ledger set-lock`
- Update the journal lock state with an explicit approval reference; JSON output mirrors the most recent lock snapshot.

```shell
codex ledger set-lock --journal-id jnl-gl --fiscal-year 2024 --period 5 --action close --approval-ref CLI-APR
codex ledger set-lock --journal-id jnl-gl --fiscal-year 2024 --period 5 --action close --approval-ref CLI-APR --format json
```

#### `codex ledger reconciliation summary`
- Surface ingest dedupe metrics, approval backlog, transaction duplicates, and persisted telemetry counters.

```shell
codex ledger reconciliation summary
codex ledger reconciliation summary --format json
```

#### `codex ledger go-live checklist`
- Run the readiness checklist covering locks, reconciliation coverage, approvals backlog, monitoring stubs, and telemetry hygiene reminders.

```shell
codex ledger go-live-checklist
```

- The checklist now points to `codex ledger entries --format json` for export validation, calls out monitoring TODOs (wire metrics dashboards and pager alerts), and prints a telemetry reset reminder keyed to `<CODEX_HOME>/accounting/telemetry.json`.

#### `codex tenancy list --stream-reconciliation`
- Stream three reconciliation ticks (or view company roster) and monitor the persisted telemetry path.

```shell
codex tenancy list --stream-reconciliation
codex tenancy list --stream-reconciliation --json
```

`--json` streams two newline-delimited JSON snapshots suitable for automation; each tick surfaces coverage, backlog counts, ingest dedupe stats, telemetry counters, and the resolved telemetry path.

Counters persist under `<CODEX_HOME>/accounting/telemetry.json`. Delete that file to reset demo metrics before another run; the CLI recreates it automatically and logs a warning if existing data is corrupt.

- Inside the TUI, press <kbd>F6</kbd> to open a ledger overlay or <kbd>F7</kbd> for the reconciliation dashboard with live status bars.


### Esc–Esc to edit a previous message

When the chat composer is empty, press Esc to prime “backtrack” mode. Press Esc again to open a transcript preview highlighting the last user message; press Esc repeatedly to step to older user messages. Press Enter to confirm and Codex will fork the conversation from that point, trim the visible transcript accordingly, and pre‑fill the composer with the selected user message so you can edit and resubmit it.

In the transcript preview, the footer shows an `Esc edit prev` hint while editing is active.

### `--cd`/`-C` flag

Sometimes it is not convenient to `cd` to the directory you want Codex to use as the "working root" before running Codex. Fortunately, `codex` supports a `--cd` option so you can specify whatever folder you want. You can confirm that Codex is honoring `--cd` by double-checking the **workdir** it reports in the TUI at the start of a new session.

### Shell completions

Generate shell completion scripts via:

```shell
codex completion bash
codex completion zsh
codex completion fish
```

### Experimenting with the Codex Sandbox

To test to see what happens when a command is run under the sandbox provided by Codex, we provide the following subcommands in Codex CLI:

```
# macOS
codex sandbox macos [--full-auto] [COMMAND]...

# Linux
codex sandbox linux [--full-auto] [COMMAND]...

# Legacy aliases
codex debug seatbelt [--full-auto] [COMMAND]...
codex debug landlock [--full-auto] [COMMAND]...
```

### Selecting a sandbox policy via `--sandbox`

The Rust CLI exposes a dedicated `--sandbox` (`-s`) flag that lets you pick the sandbox policy **without** having to reach for the generic `-c/--config` option:

```shell
# Run Codex with the default, read-only sandbox
codex --sandbox read-only

# Allow the agent to write within the current workspace while still blocking network access
codex --sandbox workspace-write

# Danger! Disable sandboxing entirely (only do this if you are already running in a container or other isolated env)
codex --sandbox danger-full-access
```

The same setting can be persisted in `~/.codex/config.toml` via the top-level `sandbox_mode = "MODE"` key, e.g. `sandbox_mode = "workspace-write"`.

## Code Organization

This folder is the root of a Cargo workspace. It contains quite a bit of experimental code, but here are the key crates:

- [`core/`](./core) contains the business logic for Codex. Ultimately, we hope this to be a library crate that is generally useful for building other Rust/native applications that use Codex.
- [`exec/`](./exec) "headless" CLI for use in automation.
- [`tui/`](./tui) CLI that launches a fullscreen TUI built with [Ratatui](https://ratatui.rs/).
- [`cli/`](./cli) CLI multitool that provides the aforementioned CLIs via subcommands.
