$ErrorActionPreference = "Stop"

Write-Host "=== Checking formatting ==="
cargo fmt --all -- --check

Write-Host "=== Running cargo check ==="
cargo check --all-targets

Write-Host "=== Running clippy ==="
cargo clippy --all-targets -- -D warnings

Write-Host "=== Running tests ==="
cargo test --all

if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    Write-Host "=== Running cargo deny ==="
    cargo deny check
} else {
    Write-Host "=== [WARN] cargo-deny is not installed. Skipping... ==="
}

if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    Write-Host "=== Running cargo audit ==="
    cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2026-0192 --ignore RUSTSEC-2026-0009 --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
} else {
    Write-Host "=== [WARN] cargo-audit is not installed. Skipping... ==="
}

if (Get-Command typos -ErrorAction SilentlyContinue) {
    Write-Host "=== Running typos ==="
    typos .
} else {
    Write-Host "=== [WARN] typos is not installed. Skipping... ==="
}

if (Get-Command lychee -ErrorAction SilentlyContinue) {
    Write-Host "=== Running lychee ==="
    lychee --cache --max-cache-age 1d --exclude-loopback --exclude "spotify.*localhost" --exclude-path .agents --exclude-path target --verbose "**/*.md"
} else {
    Write-Host "=== [WARN] lychee is not installed. Skipping... ==="
}

Write-Host "=== Compiling in release mode ==="
cargo build --release

Write-Host "=== Generating documentation ==="
cargo doc --no-deps

Write-Host "=== All done! Ready to push ==="
