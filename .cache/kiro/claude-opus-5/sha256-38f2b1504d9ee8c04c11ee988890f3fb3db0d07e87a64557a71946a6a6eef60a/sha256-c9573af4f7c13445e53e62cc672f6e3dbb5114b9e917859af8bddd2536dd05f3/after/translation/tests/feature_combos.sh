#!/usr/bin/env bash
# Phase D — enumerate every cargo feature combination from Cargo.toml and run
# `cargo check` + the full differential suite + symbol parity for each.
#
# This crate declares no [features] and no optional dependencies, so the
# combination set is {default} == {no-default-features}. The script derives that
# mechanically rather than assuming it, so it stays correct if features are
# added later.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

manifest="Cargo.toml"

# Feature names declared in [features], excluding "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' "$manifest" | grep -v '^default$'
)

# Optional dependencies are implicit features too.
mapfile -t OPTIONAL < <(
  awk '
    /^\[dependencies\]/ {ind=1; next}
    /^\[/               {ind=0}
    ind && /optional[[:space:]]*=[[:space:]]*true/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' "$manifest"
)

ALL=("${FEATURES[@]}" "${OPTIONAL[@]}")
n=${#ALL[@]}
echo "declared features: ${n} ${ALL[*]:-(none)}"

# Build the combination list: the default build, plus the full power set of the
# declared features under --no-default-features.
COMBOS=("DEFAULT" "NONE")
if [ "$n" -gt 0 ]; then
  total=$(( 1 << n ))
  for ((mask = 1; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( (mask >> i) & 1 )); then combo="${combo:+$combo,}${ALL[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "combinations to verify: ${#COMBOS[@]} -> ${COMBOS[*]}"
echo

rc=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT) args=() ;;
    NONE)    args=(--no-default-features) ;;
    *)       args=(--no-default-features --features "$combo") ;;
  esac

  echo "=============================================================="
  echo "combination: $combo   (cargo ${args[*]:-<default>})"
  echo "=============================================================="

  if ! timeout 300 cargo check "${args[@]}" 2>&1 | tail -3; then
    echo "  cargo check FAILED"; rc=1; continue
  fi

  if ! timeout 300 cargo build --release --lib "${args[@]}" >/dev/null 2>&1; then
    echo "  cargo build --release FAILED"; rc=1; continue
  fi

  if ! timeout 300 bash tests/symbol_parity.sh 2>&1 | tail -1; then
    echo "  symbol parity FAILED"; rc=1
  fi

  out=$(timeout 600 cargo test --release "${args[@]}" 2>&1)
  echo "$out" | grep -E 'test result|stdout_diff result'
  if echo "$out" | grep -qE 'FAILED|error:'; then
    echo "  TESTS FAILED for combination $combo"
    echo "$out" | grep -E '^    [a-z]' | sort -u
    rc=1
  fi
  echo
done

if [ "$rc" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASS (${#COMBOS[@]} verified)"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$rc"
