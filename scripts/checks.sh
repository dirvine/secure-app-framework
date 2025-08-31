#!/usr/bin/env bash
set -euo pipefail

export CARGO_TERM_COLOR=never

echo "Running fmt check..."
cargo fmt --all -- --check

echo "Running clippy (global policy)..."
cargo clippy --all-features -- -D clippy::panic -D clippy::unwrap_used -D clippy::expect_used

echo "Running cargo-deny (licenses & bans)..."
cargo deny check

echo "Running cargo-audit (RustSec advisories)..."
cargo audit

echo "Running tests..."
cargo test --workspace

echo "All checks passed! ✅"
