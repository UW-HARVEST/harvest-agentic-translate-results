#!/usr/bin/env bash
set -euo pipefail

timeout 600 cargo check
timeout 600 cargo build --release
timeout 600 cargo test

# Cargo.toml declares no features, so default and no-default-features are the
# complete set of distinct Cargo feature selections.
timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --no-default-features
