#!/usr/bin/env bash
# Differential test runner.
#
# `cargo test` does NOT build `crate-type = ["cdylib"]` artifacts, so the .so
# MUST be built explicitly before the tests dlopen it. Skipping this step makes
# the whole suite vacuous (it would load a stale .so). The harness also enforces
# this with an mtime check, but building here is the real fix.

set -euo pipefail
cd "$(dirname "$0")"

CARGO_FLAGS="--offline"

# Enumerate feature combinations. This crate declares no [features], so the only
# combination is the default one; the loop is written generically so that adding
# features later is automatically covered.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{split($0,a,"=");gsub(/ /,"",a[1]); if(a[1]!="default") print a[1]}' Cargo.toml || true)

run_combo() {
  local desc="$1"; shift
  echo "=============================================================="
  echo "== Feature combo: ${desc}"
  echo "=============================================================="
  # 1. Build the cdylib so the tests load a CURRENT .so.
  cargo build $CARGO_FLAGS "$@"
  # 2. Run the differential tests against it.
  cargo test $CARGO_FLAGS "$@" -- --test-threads=4
}

run_combo "default"

if [ -n "${FEATURES}" ]; then
  run_combo "no-default-features" --no-default-features
  for f in ${FEATURES}; do
    run_combo "no-default + ${f}" --no-default-features --features "$f"
  done
  # All features at once.
  run_combo "all-features" --all-features
else
  echo "=============================================================="
  echo "== Cargo.toml declares no [features]; 'default' is the only"
  echo "== configuration. Verifying --no-default-features/--all-features"
  echo "== are still equivalent no-ops."
  echo "=============================================================="
  run_combo "no-default-features (no features declared)" --no-default-features
  run_combo "all-features (no features declared)" --all-features
fi

echo
echo "ALL FEATURE COMBINATIONS PASSED"
