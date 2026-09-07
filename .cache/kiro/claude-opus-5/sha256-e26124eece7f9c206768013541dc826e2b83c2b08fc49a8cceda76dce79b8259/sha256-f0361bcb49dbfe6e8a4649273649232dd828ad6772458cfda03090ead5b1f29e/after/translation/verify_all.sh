#!/usr/bin/env bash
# Phase D helper: enumerate every feature combination declared in Cargo.toml and
# run `cargo check` + the full test suite for each. Also re-verifies `nm -D`
# symbol parity between the C and Rust shared objects.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)"
R_SO="$ROOT/translation/target/release/libdoubleneg_lib.so"

fail=0

# ---------------------------------------------------------------------------
# 1. enumerate features
# ---------------------------------------------------------------------------
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/ {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
     split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
     if (a[1] != "default") print a[1]
  }' Cargo.toml)

echo "== declared features =="
if [ -z "$FEATURES" ]; then
  echo "(none — Cargo.toml declares no [features] section)"
else
  echo "$FEATURES"
fi

# Build the combination list: always include the default build and the
# no-default-features build; add the powerset of declared features.
declare -a COMBOS=("__default__" "__nodefault__")
if [ -n "$FEATURES" ]; then
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FARR[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

# ---------------------------------------------------------------------------
# 2. check + test each combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__)   args=() ; label="default" ;;
    __nodefault__) args=(--no-default-features) ; label="--no-default-features" ;;
    *)             args=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac

  echo
  echo "===== $label ====="

  if ! timeout 600 cargo check "${args[@]}" >/tmp/dnv-check.log 2>&1; then
    echo "FAIL: cargo check ($label)"; tail -20 /tmp/dnv-check.log; fail=1; continue
  fi
  echo "cargo check: ok"

  if ! timeout 600 cargo build --release "${args[@]}" >/tmp/dnv-build.log 2>&1; then
    echo "FAIL: cargo build --release ($label)"; tail -20 /tmp/dnv-build.log; fail=1; continue
  fi
  echo "cargo build --release: ok"

  # symbol parity for this configuration
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "FAIL: symbols missing from the Rust .so ($label):"; echo "$missing"; fail=1
  else
    echo "nm -D symbol parity: ok (0 missing)"
  fi

  if ! timeout 600 cargo test "${args[@]}" >/tmp/dnv-test.log 2>&1; then
    echo "FAIL: cargo test ($label)"; grep -E '^(test result|failures:|    )' /tmp/dnv-test.log | tail -30; fail=1; continue
  fi
  grep -E '^test result' /tmp/dnv-test.log | sed 's/^/  /'
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"
