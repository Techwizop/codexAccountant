#!/bin/bash
# Generate TypeScript bindings from Rust protocol definitions
# Run this script after installing Rust/Cargo

set -euo pipefail

echo "=== Codex Accounting - Generate TypeScript Bindings ==="
echo ""

# Check if cargo is installed
echo "Checking for Rust/Cargo installation..."
if ! command -v cargo &> /dev/null; then
    echo "✗ Error: Cargo not found!"
    echo ""
    echo "Please install Rust from https://rustup.rs/"
    echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

CARGO_VERSION=$(cargo --version)
echo "✓ Found: $CARGO_VERSION"
echo ""

# Navigate to protocol-ts directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
PROTOCOL_TS_DIR="$REPO_ROOT/codex-rs/protocol-ts"
BINDINGS_OUTPUT="$REPO_ROOT/apps/codex-gui/bindings"

echo "Generating TypeScript bindings..."
echo "  Source: codex-rs/app-server-protocol/src/protocol.rs"
echo "  Output: apps/codex-gui/bindings/"
echo ""

# Create bindings directory if it doesn't exist
mkdir -p "$BINDINGS_OUTPUT"
echo "✓ Created bindings directory"

# Run the code generator
cd "$PROTOCOL_TS_DIR"
cargo run --bin codex-protocol-ts -- --out "$BINDINGS_OUTPUT"

echo ""
echo "✓ Bindings generated successfully!"
echo ""

# List generated files
echo "Generated files:"
ls -1 "$BINDINGS_OUTPUT"/*.ts | xargs -n1 basename | sed 's/^/  - /'

echo ""
echo "Next steps:"
echo "  1. Review the generated types in apps/codex-gui/bindings/"
echo "  2. Update apps/codex-gui/src/types/protocol.ts to import from bindings"
echo "  3. Run 'pnpm typecheck' in apps/codex-gui/ to verify"
echo "  4. Start the app server and web UI for end-to-end testing"
