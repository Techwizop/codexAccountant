# Generate TypeScript bindings from Rust protocol definitions
# Run this script after installing Rust/Cargo

$ErrorActionPreference = "Stop"

Write-Host "=== Codex Accounting - Generate TypeScript Bindings ===" -ForegroundColor Cyan
Write-Host ""

# Check if cargo is installed
Write-Host "Checking for Rust/Cargo installation..." -ForegroundColor Yellow
try {
    $cargoVersion = cargo --version
    Write-Host "✓ Found: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "✗ Error: Cargo not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
    Write-Host "Or run: winget install Rustlang.Rustup" -ForegroundColor Yellow
    exit 1
}

Write-Host ""

# Navigate to protocol-ts directory
$scriptDir = Split-Path -Parent $PSCommandPath
$repoRoot = Split-Path -Parent $scriptDir
$protocolTsDir = Join-Path $repoRoot "codex-rs\protocol-ts"
$bindingsOutput = Join-Path $repoRoot "apps\codex-gui\bindings"

Write-Host "Generating TypeScript bindings..." -ForegroundColor Yellow
Write-Host "  Source: codex-rs/app-server-protocol/src/protocol.rs" -ForegroundColor Gray
Write-Host "  Output: apps/codex-gui/bindings/" -ForegroundColor Gray
Write-Host ""

# Create bindings directory if it doesn't exist
if (!(Test-Path $bindingsOutput)) {
    New-Item -ItemType Directory -Path $bindingsOutput | Out-Null
    Write-Host "✓ Created bindings directory" -ForegroundColor Green
}

# Run the code generator
Push-Location $protocolTsDir
try {
    cargo run --bin codex-protocol-ts -- --out $bindingsOutput
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ Bindings generated successfully!" -ForegroundColor Green
        Write-Host ""
        
        # List generated files
        Write-Host "Generated files:" -ForegroundColor Cyan
        Get-ChildItem $bindingsOutput -Filter "*.ts" | ForEach-Object {
            Write-Host "  - $($_.Name)" -ForegroundColor Gray
        }
        
        Write-Host ""
        Write-Host "Next steps:" -ForegroundColor Yellow
        Write-Host "  1. Review the generated types in apps/codex-gui/bindings/" -ForegroundColor Gray
        Write-Host "  2. Update apps/codex-gui/src/types/protocol.ts to import from bindings" -ForegroundColor Gray
        Write-Host "  3. Run 'pnpm typecheck' in apps/codex-gui/ to verify" -ForegroundColor Gray
        Write-Host "  4. Start the app server and web UI for end-to-end testing" -ForegroundColor Gray
        
    } else {
        Write-Host "✗ Error: Binding generation failed!" -ForegroundColor Red
        exit 1
    }
} finally {
    Pop-Location
}
