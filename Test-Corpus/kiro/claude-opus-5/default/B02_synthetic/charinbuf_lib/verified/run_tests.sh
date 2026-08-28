#!/usr/bin/env bash
# Differential test driver: builds the C .so, builds the Rust cdylib, compares
# exported symbols, then runs the differential suite for every feature
# combination declared in Cargo.toml.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/charinbuf_verify.log
: > "$LOG"

fail=0

# ---- 1. Enumerate feature combinations -------------------------------------
# Read [features] from Cargo.toml; anything other than `default` is optional.
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0,a,"="); gsub(/[[:space:]]/,"",a[1]); if (a[1]!="default") print a[1] }' \
    "$CRATE/Cargo.toml"
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "No optional features declared; single configuration." | tee -a "$LOG"
  COMBOS=("")
else
  n=${#FEATURES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "Feature combinations: ${#COMBOS[@]}" | tee -a "$LOG"

# ---- 2. Build the C shared library -----------------------------------------
echo "== building C ==" | tee -a "$LOG"
(
  cd "$ROOT/c_src" &&
    mkdir -p build && cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
    cmake --build .
) >>"$LOG" 2>&1 || {
  echo "C build FAILED (see $LOG)"
  exit 1
}
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "C .so: $C_SO" | tee -a "$LOG"

# ---- 3. cargo check / build / symbols / test per combination ---------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<default>"
    FLAGS=()
  else
    label="$combo"
    FLAGS=(--no-default-features --features "$combo")
  fi
  echo "" | tee -a "$LOG"
  echo "=================== features: $label ===================" | tee -a "$LOG"

  echo "-- cargo check" | tee -a "$LOG"
  if ! (cd "$CRATE" && timeout 600 cargo check "${FLAGS[@]}") >>"$LOG" 2>&1; then
    echo "  CHECK FAILED [$label]"
    fail=1
    continue
  fi

  # Build the cdylib the tests will dlopen.
  echo "-- cargo build" | tee -a "$LOG"
  if ! (cd "$CRATE" && timeout 600 cargo build "${FLAGS[@]}") >>"$LOG" 2>&1; then
    echo "  BUILD FAILED [$label]"
    fail=1
    continue
  fi

  RUST_SO="$CRATE/target/debug/libcharinbuf_lib.so"
  echo "-- symbol comparison" | tee -a "$LOG"
  c_syms=$(nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="B"||$2=="D"||$2=="W"{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$RUST_SO" | awk '$2=="T"||$2=="B"||$2=="D"||$2=="W"{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "  MISSING EXPORTS in Rust .so [$label]:"
    echo "$missing" | sed 's/^/    /'
    fail=1
  else
    echo "  all $(echo "$c_syms" | wc -l) C exports present in Rust .so" | tee -a "$LOG"
  fi

  echo "-- cargo test" | tee -a "$LOG"
  if (cd "$CRATE" && timeout 600 cargo test "${FLAGS[@]}" -- --test-threads=1) >>"$LOG" 2>&1; then
    echo "  TESTS PASSED [$label]"
  else
    echo "  TESTS FAILED [$label] — see $LOG"
    grep -E "^(test |failures:|assertion|  left|  right| *charinbuf)" "$LOG" | tail -40
    fail=1
  fi
done

echo ""
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
