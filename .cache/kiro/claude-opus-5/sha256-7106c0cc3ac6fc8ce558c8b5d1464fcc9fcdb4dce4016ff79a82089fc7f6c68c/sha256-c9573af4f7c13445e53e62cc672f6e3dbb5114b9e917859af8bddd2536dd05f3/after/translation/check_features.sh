#!/usr/bin/env bash
# Phase D -- enumerate every feature combination from Cargo.toml and run
# `cargo check` + the full differential suite under each one.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] table, ignoring "default".
FEATURES=$(awk '
  /^\[features\]/       { inf=1; next }
  /^\[/                 { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }
' Cargo.toml | sort -u)

N=$(printf '%s\n' "$FEATURES" | grep -c . || true)
echo "optional features declared in Cargo.toml: $N"
if [ "$N" -gt 0 ]; then printf '  - %s\n' $FEATURES; fi
echo

# Build the combination list: default, no-default-features, then the full power
# set of the optional features (with and without default features).
COMBOS=()
COMBOS+=("")                                # default build
COMBOS+=("--no-default-features")
if [ "$N" -gt 0 ]; then
  FARR=($FEATURES)
  total=$((1 << N))
  for ((mask = 1; mask < total; mask++)); do
    sel=""
    for ((i = 0; i < N; i++)); do
      if (( (mask >> i) & 1 )); then sel="$sel,${FARR[$i]}"; fi
    done
    sel="${sel#,}"
    COMBOS+=("--features $sel")
    COMBOS+=("--no-default-features --features $sel")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"
echo

rc=0
for combo in "${COMBOS[@]}"; do
  flags="$combo"
  label="${flags:-<default>}"
  printf 'combo %-45s ' "$label"

  if ! timeout 300 cargo check $flags >/dev/null 2>&1; then
    echo "CHECK FAILED"; rc=1; continue
  fi
  # Rebuild the cdylib under this combo so the tests load the right artifact.
  if ! timeout 300 cargo build --lib $flags --target-dir target/ffi-so >/dev/null 2>&1; then
    echo "BUILD FAILED"; rc=1; continue
  fi
  if ! timeout 600 cargo test --tests $flags >/dev/null 2>&1; then
    echo "TESTS FAILED"; rc=1; continue
  fi
  if ! ./check_symbols.sh target/ffi-so/debug/libfallcalc_lib.so >/dev/null 2>&1; then
    echo "SYMBOLS FAILED"; rc=1; continue
  fi
  echo "ok (check + tests + symbol parity)"
done

echo
[ "$rc" -eq 0 ] && echo "all ${#COMBOS[@]} feature combination(s) pass" || echo "FAILURES present"
exit $rc
