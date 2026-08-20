#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared objects, diffs their
# exported symbols and runs the differential test suite for EVERY feature
# combination declared in Cargo.toml.
#
# Usage: ./check_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
LOG_DIR=${TMPDIR:-/tmp}/driver_verify.$$
mkdir -p "$LOG_DIR"
FAILED=0

say() { printf '\n=== %s ===\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate every feature combination (powerset of [features] in Cargo.toml)
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No features exist: --no-default-features, default and --all-features are
  # all the same single configuration; run each spelling anyway.
  COMBOS=("--no-default-features" "" "--all-features")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    if [ -z "$combo" ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features $combo")
    fi
  done
  COMBOS+=("" "--all-features")
fi

say "feature combinations to verify (${#COMBOS[@]})"
for c in "${COMBOS[@]}"; do echo "  cargo <cmd> ${c:-<default>}"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared library
# ---------------------------------------------------------------------------
say "building C shared library"
(
  mkdir -p c_src/build && cd c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
    cmake --build .
) >"$LOG_DIR/cmake.log" 2>&1 || {
  fail "C build (see $LOG_DIR/cmake.log)"
  tail -20 "$LOG_DIR/cmake.log"
  exit 1
}
C_SO="$ROOT/c_src/build/libdriver.so"
nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u >"$LOG_DIR/c.syms"
echo "C .so exports $(wc -l <"$LOG_DIR/c.syms") symbol(s)"

# ---------------------------------------------------------------------------
# 3. Per-combination: check, build, symbol diff, tests
# ---------------------------------------------------------------------------
i=0
for combo in "${COMBOS[@]}"; do
  i=$((i + 1))
  tag="combo$i"
  say "[$tag] cargo check ${combo:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo check --offline --all-targets $combo >"$LOG_DIR/$tag.check.log" 2>&1; then
    fail "[$tag] cargo check"
    tail -30 "$LOG_DIR/$tag.check.log"
    continue
  fi
  grep -E "^(warning|error)" "$LOG_DIR/$tag.check.log" | sort -u | head -10

  say "[$tag] cargo build (cdylib) ${combo:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo build --offline $combo >"$LOG_DIR/$tag.build.log" 2>&1; then
    fail "[$tag] cargo build"
    tail -30 "$LOG_DIR/$tag.build.log"
    continue
  fi

  RUST_SO="$ROOT/target/debug/libdriver.so"
  say "[$tag] symbol parity"
  nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u >"$LOG_DIR/$tag.rust.syms"
  missing=$(comm -23 "$LOG_DIR/c.syms" "$LOG_DIR/$tag.rust.syms")
  if [ -n "$missing" ]; then
    fail "[$tag] symbols exported by C but missing from Rust:"
    echo "$missing"
  else
    echo "OK: all $(wc -l <"$LOG_DIR/c.syms") C symbol(s) exported by the Rust .so"
  fi
  undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' |
    grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^__cxa|^_Unwind|^statx$|^gettid$')
  if [ -n "$undef" ]; then
    fail "[$tag] non-libc undefined symbols in Rust .so:"
    echo "$undef"
  else
    echo "OK: no non-libc undefined symbols"
  fi

  say "[$tag] cargo test ${combo:-<default>}"
  # shellcheck disable=SC2086
  if ! timeout 600 cargo test --offline $combo >"$LOG_DIR/$tag.test.log" 2>&1; then
    fail "[$tag] cargo test"
    grep -E "^(test result|failures:|---- )" -A2 "$LOG_DIR/$tag.test.log" | head -60
    continue
  fi
  grep -E "^test result" "$LOG_DIR/$tag.test.log"
done

say "summary"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CHECKS PASSED (${#COMBOS[@]} feature combination(s))"
else
  echo "THERE WERE FAILURES — logs in $LOG_DIR"
fi
exit "$FAILED"
