# TypeScript Bindings Generation Scripts

This directory contains helper scripts for generating TypeScript type definitions from the Rust protocol crate.

## Overview

The Codex Accounting project uses Rust for the backend and TypeScript for the frontend. To maintain type safety across the boundary, we auto-generate TypeScript types from the Rust source of truth using the `ts-rs` library.

## Scripts

### `generate-bindings.ps1` (Windows PowerShell)

```powershell
.\scripts\generate-bindings.ps1
```

Generates TypeScript bindings on Windows systems.

### `generate-bindings.sh` (Linux/macOS)

```bash
chmod +x ./scripts/generate-bindings.sh
./scripts/generate-bindings.sh
```

Generates TypeScript bindings on Unix-like systems.

## Prerequisites

Both scripts require:
- **Rust toolchain** (rustc + cargo) installed
- Repository cloned locally

### Installing Rust

**Windows:**
```powershell
winget install Rustlang.Rustup
# Or download from https://rustup.rs/
```

**Linux/macOS:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Verify installation:**
```bash
cargo --version
rustc --version
```

## What These Scripts Do

1. ✅ Check for Rust/Cargo installation
2. ✅ Create `apps/codex-gui/bindings/` directory (if needed)
3. ✅ Run `cargo run --bin codex-protocol-ts -- --out apps/codex-gui/bindings`
4. ✅ List all generated `.ts` files
5. ✅ Display next steps for wiring into the web UI

## Generated Output

After running, you'll see files like:

```
apps/codex-gui/bindings/
├── index.ts                           # Re-exports all types
├── LedgerCompany.ts
├── LedgerAccount.ts
├── LedgerJournalEntry.ts
├── LedgerJournalLine.ts
├── LedgerListCompaniesParams.ts
├── LedgerListCompaniesResponse.ts
└── ... (and more)
```

## Integration Steps

Once bindings are generated:

1. **Replace placeholder types** in `apps/codex-gui/src/types/protocol.ts`:

   ```typescript
   // Before (placeholder types):
   export interface LedgerCompany {
     id: string
     name: string
     // ...
   }

   // After (use generated bindings):
   export * from '@/bindings'
   ```

2. **Verify types compile**:
   ```bash
   cd apps/codex-gui
   pnpm typecheck
   ```

3. **Run linter**:
   ```bash
   pnpm lint
   ```

4. **Test the application**:
   ```bash
   # Terminal 1: Start app server
   cd codex-rs
   CODEX_LEDGER_IN_MEMORY=1 cargo run --features ledger --bin codex-app-server

   # Terminal 2: Start web UI
   cd apps/codex-gui
   pnpm dev
   ```

## Troubleshooting

### "cargo: command not found"

**Solution**: Install Rust toolchain (see Prerequisites above)

### "error: could not find `Cargo.toml`"

**Solution**: Make sure you're running the script from the repository root or use the full path

### "Permission denied" (Linux/macOS)

**Solution**: Make the script executable:
```bash
chmod +x ./scripts/generate-bindings.sh
```

### Bindings are stale/outdated

**Solution**: Re-run the generation script whenever you modify `codex-rs/app-server-protocol/src/protocol.rs`

## When to Regenerate Bindings

Regenerate TypeScript bindings whenever you:
- ✅ Add new protocol types in Rust
- ✅ Modify existing protocol types
- ✅ Add new RPC methods
- ✅ Change field names or types
- ✅ Pull latest changes that affect protocol definitions

## Alternative: DevContainer

If you prefer not to install Rust locally, use the VS Code DevContainer:

1. Open repository in VS Code
2. Click "Reopen in Container" (or Ctrl+Shift+P → "Dev Containers: Reopen in Container")
3. Inside container, run:
   ```bash
   cd codex-rs/protocol-ts
   cargo run --bin codex-protocol-ts -- --out ../../apps/codex-gui/bindings
   ```

## Manual Generation

If the scripts don't work for your setup, generate manually:

```bash
cd codex-rs/protocol-ts
cargo run --bin codex-protocol-ts -- --out ../../apps/codex-gui/bindings
```

## More Information

- **Rust Protocol Source**: `codex-rs/app-server-protocol/src/protocol.rs`
- **Generator Tool**: `codex-rs/protocol-ts/src/main.rs`
- **Web UI Types**: `apps/codex-gui/src/types/protocol.ts`
- **Phase 3 Status**: `PHASE_3_CONTINUATION_STATUS.md`
