#!/usr/bin/env bash
# Phase D driver: enumerate every build-time feature combination mechanically
# from Cargo.toml, then `cargo check` + `cargo test` each one.
#
# Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"

fail=0

# --- 1. enumerate the [features] table ------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

echo "== declared features (excluding \"default\"): ${#FEATURES[@]}"
for f in "${FEATURES[@]}"; do echo "   - $f"; done
[ "${#FEATURES[@]}" -eq 0 ] && echo "   (none — Cargo.toml has no [features] table)"

# --- 2. build the power set ------------------------------------------------
COMBOS=("")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  COMBOS=()
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "== feature combinations to verify: ${#COMBOS[@]}"

# --- 3. the C reference ----------------------------------------------------
echo
echo "== building the C reference shared library"
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "!! C build FAILED"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
echo "   $C_SO"

# --- 4. check + test every combination ------------------------------------
for combo in "${COMBOS[@]}"; do
  label=${combo:-"<no features>"}
  echo
  echo "############ features: $label"

  echo "-- cargo check"
  if ! timeout 600 cargo check --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -5; then
    echo "!! check FAILED for $label"; fail=1; continue
  fi

  echo "-- cargo build --release (cdylib)"
  if ! timeout 600 cargo build --release --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -5; then
    echo "!! build FAILED for $label"; fail=1; continue
  fi

  echo "-- symbol diff (C exports missing from Rust)"
  R_SO=target/release/libhsl_to_rgb_lib.so
  c_syms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "!! MISSING from the Rust .so:"; echo "$missing"; fail=1
  else
    echo "   OK: symbol diff is empty ($(echo "$c_syms" | wc -l) exported symbol(s))"
  fi
  nonlibc=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
    | grep -v '@GLIBC' | grep -v '@GCC' \
    | grep -vE '^(_ITM_|__gmon_start__|_Unwind_|gettid|statx|__cxa_)' || true)
  if [ -n "$nonlibc" ]; then
    echo "!! non-libc undefined symbols in the Rust .so:"; echo "$nonlibc"; fail=1
  else
    echo "   OK: 0 missing/undefined non-libc symbols"
  fi

  echo "-- cargo test"
  if ! timeout 600 cargo test --no-default-features ${combo:+--features "$combo"} 2>&1 | tail -12; then
    echo "!! tests FAILED for $label"; fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED (${#COMBOS[@]} combination(s))"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"
