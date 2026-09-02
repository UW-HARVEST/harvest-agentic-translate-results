#!/usr/bin/env bash
# Differential-verification driver.
#
# `cargo test` does NOT build a `crate-type = ["cdylib"]` target, so the Rust
# .so must be built explicitly before each test run. The harness loads the
# RELEASE cdylib by default (the shipped artifact); an extra pass re-runs
# everything against the DEBUG cdylib via RUST_SO to prove the translation is
# profile-independent.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
TIMEOUT="${TIMEOUT:-600}"
SUITES=(phase_d_symbols phase_e_mutation_control phase_c_errors phase_b_configs)

echo "=== building C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/*.so)"
echo "C .so: $C_SO"

cd "$CRATE"

# Enumerate feature combinations from Cargo.toml. This crate declares no
# [features] table, so the only combination is the default (empty) one; the
# loop stays correct if features are ever added.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {gsub(/ /,"");split($0,a,"=");if(a[1]!="default")print a[1]}' Cargo.toml
)
echo "=== declared features: ${#FEATURES[@]} (${FEATURES[*]:-none}) ==="

FAIL=0

run_combo() {
  local label="$1"; shift
  echo
  echo "######## combo: $label   (flags: ${*:-none}) ########"

  timeout "$TIMEOUT" cargo build --release "$@" >/dev/null 2>&1 \
    || { echo "RELEASE BUILD FAILED [$label]"; FAIL=1; return; }
  timeout "$TIMEOUT" cargo build "$@" >/dev/null 2>&1 \
    || { echo "DEBUG BUILD FAILED [$label]"; FAIL=1; return; }

  for so in release debug; do
    local path="$CRATE/target/$so/libbitwriter_add_lib.so"
    [ -f "$path" ] || { echo "missing $so cdylib [$label]"; FAIL=1; continue; }
    echo "---- testing against $so cdylib ----"
    for t in "${SUITES[@]}"; do
      # The debug cdylib's only divergence is ERRORS.md row E13: rustc's
      # debug-profile ub_checks turn the C's unchecked NULL store into a panic
      # (SIGABRT) instead of SIGSEGV. Skip just that row for the debug pass.
      local skip=()
      [ "$so" = debug ] && skip=(--skip e13_null_bw_documented_only)
      RUST_SO="$path" timeout "$TIMEOUT" cargo test "$@" --test "$t" -- \
        --test-threads=4 "${skip[@]}" >/tmp/vt.log 2>&1 \
        && echo "  PASS $t ($(grep -o 'test result: ok. [0-9]* passed' /tmp/vt.log | tail -1))" \
        || { echo "  FAIL $t [$label/$so]"; tail -25 /tmp/vt.log; FAIL=1; }
    done
  done

  echo "---- symbol diff [$label] ----"
  diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
       <(nm -D --defined-only "$CRATE/target/release/libbitwriter_add_lib.so" \
           | awk '{print $3}' | sort) \
    && echo "  symbol diff EMPTY (parity reached)" \
    || { echo "  symbol diff NON-EMPTY"; FAIL=1; }
}

run_combo "default"
run_combo "no-default-features" --no-default-features
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  run_combo "no-default+$f" --no-default-features --features "$f"
done
if [ "${#FEATURES[@]}" -gt 0 ]; then
  run_combo "all-features" --all-features
fi

echo
if [ "$FAIL" -eq 0 ]; then echo "=== ALL COMBINATIONS PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit "$FAIL"
