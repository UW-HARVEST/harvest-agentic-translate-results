#!/usr/bin/env bash
# Differential verification driver: C .so vs. Rust .so.
#
#   ./verify.sh              run every feature combination
#   ./verify.sh <combo>      run one combination ("" means default features)
#
# Cargo.toml has no [features], so the complete set of valid build-time
# configurations is: default, --no-default-features, --all-features (all three
# are the same build, and all three are exercised).
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
LOG=${TMPDIR:-/tmp}/verify-$$.log
fail=0

say() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------- C library
say "building the C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOG" 2>&1 || { tail -30 "$LOG"; echo "C BUILD FAILED"; exit 1; }
ls -l c_src/build/libdriver.so

# ------------------------------------------------- every feature combination
COMBOS=("default" "no-default-features" "all-features")

flags_for() {
  case "$1" in
    default)              echo "" ;;
    no-default-features)  echo "--no-default-features" ;;
    all-features)         echo "--all-features" ;;
    *)                    echo "--no-default-features --features $1" ;;
  esac
}

if [ $# -ge 1 ]; then COMBOS=("$@"); fi

for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  FLAGS=$(flags_for "$combo")

  say "cargo check [$combo]"
  # shellcheck disable=SC2086
  timeout 600 cargo check --offline --all-targets $FLAGS 2>&1 | tail -5 || fail=1

  say "cargo build [$combo]  (cargo test does NOT rebuild a cdylib)"
  # shellcheck disable=SC2086
  timeout 600 cargo build --offline $FLAGS 2>&1 | tail -5 || fail=1

  say "nm -D symbol diff [$combo]"
  nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort -u > "$LOG.c"
  nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort -u > "$LOG.r"
  missing=$(comm -23 "$LOG.c" "$LOG.r")
  if [ -n "$missing" ]; then
    echo "MISSING FROM RUST .so:"; echo "$missing"; fail=1
  else
    echo "OK: all $(wc -l < "$LOG.c") C symbols are exported by the Rust .so"
  fi

  say "cargo test [$combo]  (serial: the suite captures fd 1 / fd 2)"
  # shellcheck disable=SC2086
  timeout 600 cargo test --offline $FLAGS -- --test-threads=1 2>&1 | tail -70 || fail=1
done

# The C library is built unoptimized; check the optimized Rust build too, since
# `wrapping_mul` / stdio usage must behave the same at any opt-level.
say "release profile (optimized Rust .so vs. the same C .so)"
timeout 600 cargo build --offline --release 2>&1 | tail -3 || fail=1
timeout 600 cargo test --offline --release -- --test-threads=1 2>&1 \
  | grep -E "test result|FAILED" || fail=1

say "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES (see above)"; fi
exit "$fail"
