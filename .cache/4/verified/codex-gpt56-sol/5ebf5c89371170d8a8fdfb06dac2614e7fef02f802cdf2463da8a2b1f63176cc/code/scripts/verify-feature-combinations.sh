#!/usr/bin/env bash
set -euo pipefail

# Cargo.toml has no features, so the empty set is the complete combination list.
timeout 600 cargo check --no-default-features
timeout 600 cargo test --no-default-features
