# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Codex Accountant** is an AI-powered autonomous accounting platform built by extending the Codex CLI codebase with accounting-specific features. The system uses AI to process financial documents, suggest journal entries, and automate accounting workflows while maintaining human oversight through approval mechanisms.

**Status**: Phase 1-3 Complete (AI Tools, API Layer, Web UI)

## Essential Commands

### Backend (Rust)

```bash
# Build with accounting features
cd codex-rs
cargo build --features ledger

# Run tests for core accounting tools
cargo test --features ledger -p codex-core accounting

# Run app server with in-memory ledger
CODEX_LEDGER_IN_MEMORY=1 cargo run --features ledger --bin codex-app-server

# Run all tests (requires cargo-nextest)
just test

# Format code (with import sorting)
just fmt

# Auto-fix clippy warnings
just fix

# Run clippy
just clippy
```

### Frontend (React)

```bash
# Install dependencies (first time)
cd apps/codex-gui
pnpm install

# Start development server (requires app server running)
pnpm dev

# Build for production
pnpm build

# Type check
pnpm typecheck

# Lint
pnpm lint

# Format
pnpm format
```

### Full Stack Development

Terminal 1 (Backend):
```bash
cd codex-rs
CODEX_LEDGER_IN_MEMORY=1 cargo run --features ledger --bin codex-app-server
```

Terminal 2 (Frontend):
```bash
cd apps/codex-gui
pnpm dev
```

Open browser to http://localhost:3000

## Architecture Overview

### Monorepo Structure

```
CodexAccountant/
├── codex-rs/                   # Rust workspace (backend)
│   ├── core/                   # Core logic & AI tools
│   │   └── src/
│   │       ├── tools/
│   │       │   └── accounting.rs    # 7 accounting tools for AI
│   │       └── accounting/          # Document processing agent
│   ├── app-server/             # JSON-RPC API server
│   │   └── src/
│   │       ├── accounting_handlers.rs
│   │       └── codex_message_processor.rs
│   ├── app-server-protocol/    # API type definitions
│   ├── codex-ledger/           # Double-entry accounting engine
│   ├── codex-accounting-api/   # Facade for accounting operations
│   ├── codex-approvals/        # Approval workflow service
│   ├── codex-policy/           # Policy evaluation engine
│   ├── codex-reconcile/        # Bank reconciliation
│   ├── codex-audit-log/        # Audit trail
│   ├── codex-doc-store/        # Document storage
│   ├── codex-doc-ingest/       # Document ingestion
│   ├── codex-ocr/              # OCR service
│   └── cli/                    # Command-line interface
├── apps/
│   └── codex-gui/              # React web UI (Vite + TypeScript)
│       └── src/
│           ├── api/            # JSON-RPC client & React Query hooks
│           ├── components/     # UI components
│           ├── pages/          # Feature pages
│           └── lib/            # Utilities
└── docs/                       # Documentation
```

### Data Flow

1. **Document Upload** → codex-doc-ingest → codex-doc-store → S3
2. **OCR Processing** → codex-ocr → Extract text from PDF/image
3. **AI Extraction** → ChatGPT → InvoiceData with line items
4. **Entry Suggestion** → DocumentAgent → Balanced journal entry suggestion
5. **Policy Evaluation** → codex-policy → Auto-post or queue for approval
6. **Approval** → codex-approvals → Human review and approval
7. **Posting** → codex-ledger → Immutable double-entry ledger
8. **Audit** → codex-audit-log → Complete audit trail

### Feature Flag System

The accounting functionality is behind a `ledger` feature flag for conditional compilation:

```rust
// In Cargo.toml
[features]
ledger = ["codex-accounting-api", "codex-ledger", "codex-policy", "codex-approvals"]

// In code
#[cfg(feature = "ledger")]
mod accounting;
```

**When adding accounting features:**
- Use `#[cfg(feature = "ledger")]` for conditional compilation
- Update dependencies in both `[dependencies]` and `[features]` sections
- Test with `--features ledger` flag

## Key Implementation Patterns

### AI Tool Pattern

All AI tools implement `ToolHandler` trait:

```rust
use async_trait::async_trait;
use crate::tools::registry::ToolHandler;

pub struct MyAccountingTool {
    facade: Arc<LedgerFacade>,
}

#[async_trait]
impl ToolHandler for MyAccountingTool {
    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<ToolOutput, FunctionCallError> {
        // 1. Parse arguments from JSON
        // 2. Validate business rules
        // 3. Call LedgerFacade
        // 4. Return JSON response
    }
}
```

### Validation Pattern

Always validate debits == credits for journal entries:

```rust
let total_debits: i64 = lines.iter()
    .filter(|l| l.direction == "debit")
    .map(|l| l.amount_minor)
    .sum();

let total_credits: i64 = lines.iter()
    .filter(|l| l.direction == "credit")
    .map(|l| l.amount_minor)
    .sum();

if total_debits != total_credits {
    return Err(FunctionCallError::InvalidArgs(
        format!("Entry not balanced: debits={} credits={}", total_debits, total_credits)
    ));
}
```

### JSON-RPC API Pattern

Handler methods in app-server follow this pattern:

```rust
async fn handle_ledger_list_companies(
    &self,
    params: LedgerListCompaniesParams,
) -> Result<LedgerListCompaniesResponse, String> {
    // 1. Extract tenant context
    // 2. Validate parameters
    // 3. Call accounting facade
    // 4. Convert to response type
    // 5. Return result
}
```

### React Query Hook Pattern

Frontend API hooks follow this pattern:

```typescript
import { useQuery } from '@tanstack/react-query'
import { rpcClient } from '@/api/client'

export function useCompanies(params: ListCompaniesParams) {
  return useQuery({
    queryKey: ['companies', params],
    queryFn: () => rpcClient.call('ledgerListCompanies', params),
  })
}
```

## Important Architectural Decisions

### Multi-Tenancy

All accounting operations require a `TenantContext` that includes:
- `tenant_id`: Unique identifier for the organization
- `user_id`: Current user making the request
- Used for data isolation and audit trails

### Immutable Ledger

The ledger follows double-entry accounting principles:
- All entries must balance (debits == credits)
- Posted entries are immutable
- Corrections use reversal entries
- Complete audit trail maintained

### AI Integration

ChatGPT integration points:
1. **Document Extraction**: Extract structured data from invoices
2. **Entry Suggestion**: Suggest journal entries with account mappings
3. **Pattern Learning**: Learn from user corrections (future)

Prompts are templates with JSON schemas for structured output.

### Policy-Based Automation

Policy engine evaluates rules to determine if entries can be auto-posted:
- Amount thresholds
- Vendor trust levels
- Account restrictions
- Time-based rules

Returns: `AutoPost`, `RequireApproval`, or `Reject`

## Testing Conventions

### Rust Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn test_create_company_success() {
        // Arrange: Create mock dependencies
        // Act: Call the function
        // Assert: Verify expected behavior
    }
}
```

Run with: `cargo test --features ledger`

### Integration Tests

Integration tests live in separate `tests/` directories and test full workflows.

### Frontend Tests

Currently minimal - unit tests to be added using Vitest.

## Currency Handling

All monetary amounts are stored as integers in minor units (cents):
- `amount_minor: i64` (e.g., $123.45 → 12345)
- Frontend formats with `formatCurrency(amountMinor, currency, decimals)`
- Prevents floating-point precision issues

## Code Quality Standards

### Rust

- **Edition**: 2024
- **Lints**: Strict clippy lints enforced (see workspace lints in root Cargo.toml)
- **Format**: Run `just fmt` before committing
- **Error Handling**: Use `Result` and `?` operator, avoid `.unwrap()` and `.expect()`
- **Imports**: Granular imports (one per line)

### TypeScript

- **Mode**: Strict
- **Format**: Prettier with Tailwind plugin
- **Naming**: camelCase for variables/functions, PascalCase for components
- **Hooks**: Use React Query for server state, Zustand for client state

## Working with Accounting Features

### Adding a New Tool

1. Add struct and implementation in `codex-rs/core/src/tools/accounting.rs`
2. Register in `codex-rs/core/src/tools/spec.rs` (add to function definitions)
3. Update `register_accounting_tools()` in registry
4. Add unit tests
5. Build with `cargo test --features ledger -p codex-core`

### Adding an API Endpoint

1. Define request/response types in `codex-rs/app-server-protocol/src/protocol.rs`
2. Add handler method in `codex-rs/app-server/src/accounting_handlers.rs`
3. Add match case in `codex_message_processor.rs`
4. Generate TypeScript bindings: `cd app-server-protocol && cargo test`
5. Add React Query hook in `apps/codex-gui/src/api/hooks.ts`
6. Use hook in frontend component

### Adding a UI Page

1. Create page component in `apps/codex-gui/src/pages/`
2. Add route case in `App.tsx`
3. Add navigation link in `Sidebar.tsx`
4. Use API hooks from `src/api/hooks.ts`
5. Follow existing patterns for loading/error states

## Reference Documentation

For detailed implementation guides, see:
- `CURRENT_STATUS.md` - Current implementation status
- `DEVELOPMENT_PLAN.md` - Complete task breakdown
- `DEVELOPMENT_ROADMAP.md` - Architecture and specifications
- `START_HERE.md` - Quick start guide
- `apps/codex-gui/README.md` - Frontend documentation
- `PHASE_*_COMPLETE.md` - Phase completion summaries

## Common Tasks

### Fix Type Errors After Adding Accounting Feature

If you see "cannot find type" errors:
1. Check feature flag is enabled: `cargo check --features ledger`
2. Verify imports match actual crate structure
3. Ensure dependent crates have `ledger` feature

### Regenerate TypeScript Bindings

```bash
cd codex-rs/app-server-protocol
cargo test --features ledger
# Generates bindings.ts in protocol-ts/
```

### Run Full Development Stack

See "Full Stack Development" section above for terminal commands.

### Add New Accounting Service

1. Create new crate in `codex-rs/codex-{service-name}/`
2. Add to workspace members in root `Cargo.toml`
3. Add dependency in `codex-accounting-api` or `core`
4. Wire into `app-server` handlers
5. Update feature flags as needed

## Known Patterns in Codebase

### Mock vs Real Implementations

Currently, some handlers return mock data marked with `// TODO: Replace with actual implementation`. When integrating real services:
1. Remove mock response code
2. Call actual facade/service method
3. Convert types appropriately
4. Update tests to use real dependencies or mocks

### Error Propagation

- Tool handlers: Return `FunctionCallError`
- App server: Return `Result<Response, String>`
- Frontend: Handle errors in React Query hooks with `error` state

### Async Patterns

- Backend: Use `tokio` runtime with `async/await`
- All tool handlers are `async fn`
- Tests use `#[tokio::test]`

## Project Goals

**Vision**: Upload invoice PDF → AI extracts data → suggests balanced journal entry → accountant approves → posted to ledger → audit trail maintained.

**Current Progress**:
- ✅ Phase 1: AI tools and document agent
- ✅ Phase 2: JSON-RPC API layer
- ✅ Phase 3: Web UI with full workflows
- ⏳ Phase 4: Real data integration and testing
- ⏳ Phase 5: Production hardening

The system aims for 80%+ extraction accuracy, 70%+ auto-match rate for reconciliation, and <5s processing time per document.
