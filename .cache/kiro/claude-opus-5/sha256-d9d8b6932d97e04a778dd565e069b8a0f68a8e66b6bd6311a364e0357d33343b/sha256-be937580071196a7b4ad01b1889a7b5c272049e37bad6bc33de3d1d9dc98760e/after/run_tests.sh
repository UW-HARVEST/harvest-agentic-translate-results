#!/usr/bin/env bash
# Build the C .so + driver and the Rust .so + driver for one configuration, then
# run the differential test suite against that pair.
#
#   ./run_tests.sh                 # every configuration
#   ./run_tests.sh add 5           # one configuration
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

run_one() {
  local op="$1" rep="$2" features="$3" label="$4"
  local out="$ROOT/cbuild/${op}_${rep}"

  echo "======== $label (OP=$op REPEAT=$rep) ========"

  "$ROOT/build_c.sh" "$op" "$rep" >/dev/null || { echo "!!!! C build failed"; return 1; }

  ( cd "$ROOT/translation" \
    && timeout 600 cargo build --release $features >/dev/null 2>&1 ) \
    || { echo "!!!! rust build failed"; return 1; }

  # Snapshot the Rust artifacts: the next configuration overwrites target/release.
  cp "$ROOT/translation/target/release/libdriver.so" "$out/libdriver.so"
  cp "$ROOT/translation/target/release/driver"       "$out/driver_rs"

  # --- symbol parity for this configuration ---
  local cs rs
  cs="$(nm -D --defined-only "$out/libmdcore.so" | awk '{print $NF}' | sort)"
  rs="$(nm -D --defined-only "$out/libdriver.so" | awk '{print $NF}' | sort)"
  local missing extra
  missing="$(comm -23 <(echo "$cs") <(echo "$rs"))"
  extra="$(comm -13 <(echo "$cs") <(echo "$rs"))"
  if [[ -n "$missing" ]]; then
    echo "!!!! symbols in C but not Rust: $(echo "$missing" | tr '\n' ' ')"
    return 1
  fi
  if [[ -n "$extra" ]]; then
    echo "!!!! symbols in Rust but not C: $(echo "$extra" | tr '\n' ' ')"
    return 1
  fi
  echo "symbols: parity OK ($(echo "$cs" | wc -l) exported)"

  # --- differential tests ---
  ( cd "$ROOT/translation" \
    && MD_C_SO="$out/libmdcore.so" \
       MD_RUST_SO="$out/libdriver.so" \
       MD_C_BIN="$out/driver" \
       MD_RUST_BIN="$out/driver_rs" \
       MD_OP="$op" \
       MD_REPEAT="$rep" \
       timeout 600 cargo test --release --no-fail-fast $features -- --test-threads=1 ) \
    > "$out/test.log" 2>&1
  local rc=$?
  grep -E '^test result:' "$out/test.log" | sed 's/^/  /'
  if [[ $rc -ne 0 ]]; then
    echo "!!!! TESTS FAILED ($label) -- see $out/test.log"
    grep -nE "panicked|^---- |assertion|diverged|error\[" "$out/test.log" | head -n 30 | sed 's/^/  /'
    return 1
  fi
  return 0
}

fail=0
if [[ $# -eq 2 ]]; then
  run_one "$1" "$2" "--no-default-features --features $1,$2" "$1,$2" || fail=1
else
  for op in add sub mul; do
    for rep in 0 1 2 3 4 5 6 7; do
      run_one "$op" "$rep" "--no-default-features --features $op,$rep" "$op,$rep" || fail=1
    done
  done
  # #ifndef fallbacks: no features at all, and the `default` feature set.
  run_one add 5 "--no-default-features" "<no features> == add,5" || fail=1
  run_one add 5 "" "default == add,5" || fail=1
fi

if [[ $fail -eq 0 ]]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$fail"
