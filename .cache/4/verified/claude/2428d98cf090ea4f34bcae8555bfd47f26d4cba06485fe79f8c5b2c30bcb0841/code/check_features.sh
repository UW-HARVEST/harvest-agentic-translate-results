#!/usr/bin/env bash
# Phase A / Phase D — enumerate EVERY valid Cargo feature combination
# mechanically from Cargo.toml and run `cargo check` (and optionally the whole
# differential test suite) for each one.
#
#   ./check_features.sh          # cargo check for every combination
#   ./check_features.sh --test   # ... and run the full test suite too
set -uo pipefail
cd "$(dirname "$0")"

RUN_TESTS=0
[[ "${1:-}" == "--test" ]] && RUN_TESTS=1

# ---------------------------------------------------------------------------
# Enumerate the features declared in Cargo.toml ([features] section only).
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/            { in_f=1; next }
    /^\[/                      { in_f=0 }
    in_f && /^[[:space:]]*#/   { next }
    in_f && /=/                { split($0, a, "="); gsub(/[[:space:]"]/, "", a[1]);
                                 if (a[1] != "" && a[1] != "default") print a[1] }
  ' Cargo.toml
)

echo "declared features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Power set of FEATURES; always includes the empty combination.
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [[ -z "$f" ]] && continue
  new=()
  for c in "${COMBOS[@]}"; do
    if [[ -z "$c" ]]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS+=("${new[@]}")
done

echo "feature combinations to verify: ${#COMBOS[@]}"

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  echo "-----------------------------------------------------------------"
  echo "### cargo check --no-default-features --features '$label'"
  if ! cargo check --offline --all-targets --no-default-features --features "$combo" 2>&1 | tail -5; then
    echo "CHECK FAILED for '$label'"
    fail=1
  fi
  echo "### cargo build --release --no-default-features --features '$label'"
  if ! cargo build --offline --release --no-default-features --features "$combo" 2>&1 | tail -3; then
    echo "RELEASE BUILD FAILED for '$label'"
    fail=1
  fi
  ./check_symbols.sh || fail=1
  if [[ $RUN_TESTS == 1 ]]; then
    echo "### cargo test --no-default-features --features '$label'"
    if ! cargo test --offline --no-default-features --features "$combo" -- --test-threads=1 2>&1 \
         | grep -E '^(test result|error)'; then
      echo "TEST RUN PRODUCED NO RESULT LINE for '$label'"
      fail=1
    fi
    if cargo test --offline --no-default-features --features "$combo" -- --test-threads=1 2>&1 \
       | grep -qE 'FAILED|test result: FAILED'; then
      echo "TESTS FAILED for '$label'"
      fail=1
    fi
  fi
done

echo "================================================================="
if [[ $fail == 0 ]]; then
  echo "ALL ${#COMBOS[@]} FEATURE COMBINATION(S) OK"
else
  echo "FAILURES DETECTED"
fi
exit $fail
