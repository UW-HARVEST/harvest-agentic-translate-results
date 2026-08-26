#!/usr/bin/env bash
# Phase A/D — enumerate EVERY valid feature combination from Cargo.toml and run
# `cargo check` + the full differential test suite for each one.
#
# This crate declares no [features], so the enumeration yields exactly one
# combination (the empty set). The script derives that mechanically rather than
# assuming it, so it stays correct if features are ever added.
set -uo pipefail
cd "$(dirname "$0")"

# --- extract feature names from the [features] section of Cargo.toml ---------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); print
    }
  ' Cargo.toml | grep -v '^default$'
)

n=${#FEATURES[@]}
echo "features declared in Cargo.toml: $n ${FEATURES[*]:-(none)}"

# --- build the power set ----------------------------------------------------
COMBOS=()
if [ "$n" -eq 0 ]; then
  COMBOS=("")
else
  total=$((1 << n))
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( (mask >> b) & 1 )); then
        combo="${combo:+$combo,}${FEATURES[b]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "feature combinations to verify: ${#COMBOS[@]}"
echo

fail=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo "=============================================================="
  echo " combination: $label"
  echo "=============================================================="

  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  echo "--- cargo check ${args[*]}"
  if ! timeout 600 cargo check "${args[@]}" 2>&1 | tail -3; then
    echo "  CHECK FAILED for $label"; fail=1; continue
  fi

  echo "--- cargo build --release ${args[*]}"
  if ! timeout 600 cargo build --release "${args[@]}" 2>&1 | tail -2; then
    echo "  BUILD FAILED for $label"; fail=1; continue
  fi

  # Point the differential tests at the .so built for THIS combination.
  so="$PWD/target/release/libgjk_lib.so"
  if [ ! -f "$so" ]; then
    echo "  MISSING .so for $label"; fail=1; continue
  fi

  echo "--- symbol parity for $label"
  csyms="${TMPDIR:-/tmp}/c_syms.$$"
  rsyms="${TMPDIR:-/tmp}/r_syms.$$"
  nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > "$csyms"
  nm -D --defined-only "$so" | awk '{print $3}' | sort > "$rsyms"
  missing=$(comm -23 "$csyms" "$rsyms")
  if [ -n "$missing" ]; then
    echo "  MISSING SYMBOLS for $label:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  OK: all $(wc -l < "$csyms") C symbols exported"
  fi
  rm -f "$csyms" "$rsyms"

  echo "--- cargo test --release ${args[*]}"
  out=$(GJK_RUST_SO="$so" timeout 600 cargo test --release "${args[@]}" 2>&1)
  if printf '%s' "$out" | grep -q "test result: FAILED"; then
    echo "  TESTS FAILED for $label"
    printf '%s' "$out" | grep -E "^test .* FAILED|test result" | head -20
    fail=1
  else
    printf '%s' "$out" | grep -E "test result" \
      | awk -F'[ ;]' '{p+=$4; f+=$6} END {print "  tests passed="p" failed="f}'
  fi
  echo
done

echo "=============================================================="
if [ "$fail" -eq 0 ]; then
  echo " ALL ${#COMBOS[@]} FEATURE COMBINATION(S) PASSED"
else
  echo " FAILURES DETECTED"
fi
echo "=============================================================="
exit "$fail"
