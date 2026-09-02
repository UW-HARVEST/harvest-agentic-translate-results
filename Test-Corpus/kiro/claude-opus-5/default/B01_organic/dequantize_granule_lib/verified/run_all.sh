#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared objects, diffs their
# exported symbols, and runs the whole differential suite under every Cargo
# feature combination.
#
# Usage: ./run_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
FAIL=0
TIMEOUT=600

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
say "Building the C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/dev/null || { echo "C build FAILED"; exit 1; }

C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { echo "no C .so produced"; exit 1; }
echo "C  .so: $C_SO"

# ---------------------------------------------------------------------------
# Enumerate feature combinations declared in Cargo.toml. This crate declares no
# [features] table, so the enumeration yields exactly the default (empty) set;
# the loop is written generically so it keeps working if features are added.
say "Enumerating feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, "", $0); if ($0 != "default") print $0
    }
  ' "$HERE/Cargo.toml"
)
echo "declared non-default features: ${FEATURES[*]:-<none>}"

# Build the list of `cargo test` flag sets to run.
COMBOS=("" "--no-default-features")
if [ "${#FEATURES[@]}" -gt 0 ]; then
  n=${#FEATURES[@]}
  for (( mask = 1; mask < (1 << n); mask++ )); do
    sel=()
    for (( b = 0; b < n; b++ )); do
      (( mask & (1 << b) )) && sel+=("${FEATURES[$b]}")
    done
    COMBOS+=("--no-default-features --features $(IFS=,; echo "${sel[*]}")")
    COMBOS+=("--features $(IFS=,; echo "${sel[*]}")")
  done
fi
printf 'combination: %s\n' "${COMBOS[@]/#/[default]}" | sed 's/\[default\]$/[default]/'

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"

  say "cargo check  $label"
  # shellcheck disable=SC2086
  ( cd "$HERE" && timeout "$TIMEOUT" cargo check $combo --all-targets ) >/dev/null 2>&1 \
    || { echo "cargo check FAILED for $label"; FAIL=1; continue; }

  say "cargo build --release  $label"
  # shellcheck disable=SC2086
  ( cd "$HERE" && timeout "$TIMEOUT" cargo build --release $combo ) >/dev/null 2>&1 \
    || { echo "build FAILED for $label"; FAIL=1; continue; }

  R_SO="$HERE/target/release/libdequantize_granule_lib.so"
  [ -f "$R_SO" ] || { echo "no Rust .so for $label"; FAIL=1; continue; }

  say "symbol diff  $label"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/r_syms.txt
  missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
  if [ -n "$missing" ]; then
    echo "MISSING from the Rust .so:"; echo "$missing"; FAIL=1
  else
    echo "0 missing symbols ($(wc -l < /tmp/c_syms.txt) exported by the C .so)"
  fi
  undef="$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
            | grep -v -E '@GLIBC_|@GCC_|^_ITM_|^__cxa_finalize$|^__gmon_start__$' || true)"
  if [ -n "$undef" ]; then
    echo "undefined non-libc symbols in the Rust .so:"; echo "$undef"; FAIL=1
  else
    echo "0 undefined non-libc symbols"
  fi

  say "cargo test --release  $label"
  # shellcheck disable=SC2086
  ( cd "$HERE" && timeout "$TIMEOUT" cargo test --release $combo -- --test-threads=1 ) \
    || { echo "TESTS FAILED for $label"; FAIL=1; }

  # Re-run the whole suite against the DEBUG cdylib. That build has integer
  # overflow checks and `ptr::offset` UB-precondition checks enabled, so it
  # catches non-wrapping arithmetic and out-of-object address computations that
  # the release build would silently paper over.
  say "cargo test --release against the debug .so  $label"
  # shellcheck disable=SC2086
  ( cd "$HERE" && timeout "$TIMEOUT" cargo build $combo ) >/dev/null 2>&1 \
    || { echo "debug build FAILED for $label"; FAIL=1; continue; }
  # shellcheck disable=SC2086
  ( cd "$HERE" \
    && RUST_SO="$HERE/target/debug/libdequantize_granule_lib.so" \
       timeout "$TIMEOUT" cargo test --release $combo -- --test-threads=1 ) \
    || { echo "TESTS FAILED against the debug .so for $label"; FAIL=1; }
done

say "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
