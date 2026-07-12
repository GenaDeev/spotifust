#!/usr/bin/env bash

set -euo pipefail

echo "=== Checking formatting ==="
cargo fmt --all -- --check


echo "=== Running clippy ==="
cargo clippy --all-targets -- -D warnings

echo "=== Running tests ==="
cargo test --all

if command -v cargo-deny >/dev/null 2>&1; then
    echo "=== Running cargo deny ==="
    cargo deny check
else
    echo "=== [WARN] cargo-deny is not installed. Skipping... ==="
fi

if command -v cargo-audit >/dev/null 2>&1; then
    echo "=== Running cargo audit ==="
    cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2024-0436 --ignore RUSTSEC-2026-0192 --ignore RUSTSEC-2026-0009 --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
else
    echo "=== [WARN] cargo-audit is not installed. Skipping... ==="
fi

if command -v typos >/dev/null 2>&1; then
    echo "=== Running typos ==="
    typos .
else
    echo "=== [WARN] typos is not installed. Skipping... ==="
fi

if command -v lychee >/dev/null 2>&1; then
    echo "=== Running lychee ==="
    lychee --cache --max-cache-age 1d --exclude-loopback --exclude "spotify.*localhost" --exclude-path .agents --exclude-path target --verbose '**/*.md'
else
    echo "=== [WARN] lychee is not installed. Skipping... ==="
fi

if command -v markdownlint-cli2 >/dev/null 2>&1; then
    echo "=== Running markdownlint ==="
    markdownlint-cli2 '**/*.md'
else
    echo "=== [WARN] markdownlint-cli2 is not installed. Skipping... ==="
fi

echo "=== Compiling in release mode ==="
cargo build --release

echo "=== Generating documentation ==="
cargo doc --no-deps

echo "=== All done! Ready to push ==="
