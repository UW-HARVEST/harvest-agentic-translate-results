#!/usr/bin/env bash
# Phase D driver: enumerate every feature combination from Cargo.toml and run
# the full differential suite for each, against both Rust build profiles.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
FAIL=0

echo "=== C .so: $C_SO"

# --- enumerate feature combinations -----------------------------------------
# Features come from the [features] table in Cargo.toml. There is none in this
# crate, so the combination list is just the default (empty) set; the loop is
# written generically so it stays correct if features are ever added.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inside=1; next}
    /^\[/ {inside=0}
    inside && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "=== declared non-default features: ${#FEATURES[@]} (${FEATURES[*]:-none})"

COMBOS=()
COMBOS+=("__default__")
COMBOS+=("__none__")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

run_suite() {
  local label="$1"; shift
  local rust_so="$1"; shift
  echo
  echo "########## $label  (rust .so: ${rust_so##*/target/})"
  BUFFAPP_C_SO="$C_SO" BUFFAPP_RUST_SO="$rust_so" \
    timeout 600 cargo test "$@" -- --test-threads=1 2>&1 | tail -n 12
  local rc=${PIPESTATUS[0]}
  if (( rc != 0 )); then
    echo "!!!!!!!!!! FAILED: $label (rc=$rc)"
    FAIL=1
  fi
}

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) feat_args=() ; label="features: <default>" ;;
    __none__)    feat_args=(--no-default-features) ; label="features: <none>" ;;
    *)           feat_args=(--no-default-features --features "$combo") ; label="features: $combo" ;;
  esac

  # Build the cdylib in both profiles for this feature combination.
  timeout 600 cargo build --release "${feat_args[@]}" >/dev/null 2>&1 || { echo "release build failed for $label"; FAIL=1; continue; }
  cp target/release/libbuffapp_lib.so "target/rel_${combo//,/_}.so"
  timeout 600 cargo build "${feat_args[@]}" >/dev/null 2>&1 || { echo "debug build failed for $label"; FAIL=1; continue; }
  cp target/debug/libbuffapp_lib.so "target/dbg_${combo//,/_}.so"

  run_suite "$label [release .so]" "$PWD/target/rel_${combo//,/_}.so" "${feat_args[@]}"
  run_suite "$label [debug .so]"   "$PWD/target/dbg_${combo//,/_}.so" "${feat_args[@]}"
done

# --- symbol diff must be empty for every profile ----------------------------
echo
echo "########## symbol diff"
for so in target/rel_*.so target/dbg_*.so; do
  miss="$(comm -23 \
    <(nm -D --defined-only "$C_SO"  | awk '{print $NF}' | sort) \
    <(nm -D --defined-only "$so"    | awk '{print $NF}' | sort))"
  if [[ -n "$miss" ]]; then
    echo "!!!!!!!!!! $so MISSING: $miss"
    FAIL=1
  else
    echo "OK (0 missing): $so"
  fi
done

echo
if (( FAIL == 0 )); then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit $FAIL
