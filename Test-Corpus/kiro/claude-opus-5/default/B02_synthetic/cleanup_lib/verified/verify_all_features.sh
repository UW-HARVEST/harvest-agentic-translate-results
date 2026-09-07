#!/usr/bin/env bash
# Phase D — run the full differential suite under EVERY feature combination.
#
# Feature combinations are extracted from Cargo.toml via `cargo metadata`
# rather than hardcoded, so this stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(
  cargo metadata --format-version 1 --no-deps 2>/dev/null |
  python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin)["packages"][0]["features"]))'
)

# Build the power set of the declared features (empty set == --no-default-features).
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  for existing in "${COMBOS[@]}"; do
    if [ -z "$existing" ]; then COMBOS+=("$f"); else COMBOS+=("$existing,$f"); fi
  done
done

echo "declared features: ${FEATURES[*]:-<none>}"
echo "combinations to verify: ${#COMBOS[@]} (plus the default feature set)"

fail=0
run() {
  local desc="$1"; shift
  echo "=============================================================="
  echo ">>> $desc"
  echo "=============================================================="
  if ! timeout 600 cargo build --release "$@" 2>&1 | tail -2; then
    echo "!!! BUILD FAILED: $desc"; fail=1; return
  fi
  if ! timeout 600 cargo test --release "$@" 2>&1 | grep -E 'test result|FAILED|^error'; then
    echo "!!! no test output: $desc"; fail=1; return
  fi
  if timeout 600 cargo test --release "$@" 2>&1 | grep -q 'FAILED\|^error'; then
    echo "!!! TESTS FAILED: $desc"; fail=1
  fi
}

run "default feature set"
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    run "--no-default-features" --no-default-features
  else
    run "--no-default-features --features $combo" --no-default-features --features "$combo"
  fi
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$fail"
