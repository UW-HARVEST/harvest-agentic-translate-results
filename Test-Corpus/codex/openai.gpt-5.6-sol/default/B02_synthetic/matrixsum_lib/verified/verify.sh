#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

timeout 600 cargo check
timeout 600 cargo build --release
timeout 600 cargo test
timeout 600 cargo check --no-default-features
timeout 600 cargo build --release --no-default-features
timeout 600 cargo test --no-default-features
