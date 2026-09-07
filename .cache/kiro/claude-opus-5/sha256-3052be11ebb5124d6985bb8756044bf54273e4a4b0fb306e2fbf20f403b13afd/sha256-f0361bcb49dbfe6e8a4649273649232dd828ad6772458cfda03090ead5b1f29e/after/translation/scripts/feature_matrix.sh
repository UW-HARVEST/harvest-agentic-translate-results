#!/usr/bin/env bash
# Phase D - enumerate every cargo feature combination from Cargo.toml and run
# cargo check, the symbol-parity diff and the full differential test suite for
# each one.  Nothing is hard-coded: the feature list is extracted from the
# manifest, so a future [features] section is picked up automatically.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"

# --- extract feature names from the [features] section of Cargo.toml ---------
features="$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); print $0
  }' Cargo.toml | grep -v '^default$' | sort -u)"

n=0
[ -n "$features" ] && n="$(echo "$features" | wc -l)"
echo "features declared in Cargo.toml: ${n} ($(echo $features))"

# --- build the list of combinations to test ----------------------------------
combos=()
combos+=("DEFAULT")
combos+=("NONE")
if [ "$n" -gt 0 ]; then
  # every subset of the feature set (powerset), with --no-default-features
  mapfile -t farr <<< "$features"
  total=$(( 1 << ${#farr[@]} ))
  for ((m = 1; m < total; m++)); do
    set=""
    for ((b = 0; b < ${#farr[@]}; b++)); do
      if (( m & (1 << b) )); then set="$set,${farr[$b]}"; fi
    done
    combos+=("${set#,}")
  done
fi

echo "combinations to verify: ${#combos[@]}"
fail=0
for combo in "${combos[@]}"; do
  case "$combo" in
    DEFAULT) args=() ;;
    NONE)    args=(--no-default-features) ;;
    *)       args=(--no-default-features --features "$combo") ;;
  esac
  echo
  echo "############ combination: $combo  (cargo ${args[*]}) ############"
  if ! timeout 300 cargo check "${args[@]}" 2>&1 | tail -3; then
    echo "cargo check FAILED for $combo"; fail=1; continue
  fi
  if ! timeout 300 cargo build "${args[@]}" 2>&1 | tail -2; then
    echo "cargo build FAILED for $combo"; fail=1; continue
  fi
  if ! ./scripts/symbol_parity.sh debug | tail -3; then
    echo "symbol parity FAILED for $combo"; fail=1; continue
  fi
  if ! timeout 600 cargo test "${args[@]}" -- --test-threads=1 2>&1 \
        | grep -E 'test result|DIVERGENCE'; then
    echo "cargo test FAILED for $combo"; fail=1; continue
  fi
done

echo
if [ "$fail" -ne 0 ]; then echo "FEATURE MATRIX: FAIL"; exit 1; fi
echo "FEATURE MATRIX: OK"
