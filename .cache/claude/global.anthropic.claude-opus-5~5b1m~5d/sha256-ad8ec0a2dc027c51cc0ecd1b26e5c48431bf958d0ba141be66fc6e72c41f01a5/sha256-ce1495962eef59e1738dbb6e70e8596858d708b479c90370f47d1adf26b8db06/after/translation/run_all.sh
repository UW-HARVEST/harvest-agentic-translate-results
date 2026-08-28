#!/usr/bin/env bash
# Phase D driver: run the whole verification across every configuration.
#
#   1. build the C shared library
#   2. enumerate the crate's feature combinations from Cargo.toml
#   3. for each (feature combo x cargo profile): build, check the symbol diff
#      against the C .so, and run the differential suites
#   4. repeat the suites with the *other* profile's .so loaded via RUST_LIB_PATH
#   5. run the mutation-based test-power check in both profiles
#
# Usage: ./run_all.sh            (everything)
#        SKIP_MUTATION=1 ./run_all.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
fail=0
note() { printf '\n=== %s\n' "$*"; }

# ---------------------------------------------------------------- 1. C library
note "building the C shared library"
(cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
C_SO=$(ls "$ROOT"/c_src/build/*.so)
echo "C .so: $C_SO"

# ------------------------------------------------------- 2. feature combinations
# The crate declares no [features] table, so the only combinations that exist
# are the default one and its aliases; they are all enumerated (and verified to
# be equivalent) rather than assumed.
FEATURES=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml)
if [ -n "$FEATURES" ]; then
  echo "declared features: $FEATURES"
  COMBOS=()
  COMBOS+=("--no-default-features")
  for f in $FEATURES; do COMBOS+=("--no-default-features --features $f"); done
  COMBOS+=("--all-features")
  COMBOS+=("")
else
  echo "declared features: (none) -> default, --no-default-features and --all-features are aliases"
  COMBOS=("" "--no-default-features" "--all-features")
fi

# ------------------------------------- 3/4. build, symbol diff and test each combo
for combo in "${COMBOS[@]}"; do
  for profile in "" "--release"; do
    label="features='${combo:-default}' profile='${profile:-dev}'"
    note "$label"
    if ! cargo build $combo $profile >/dev/null 2>&1; then
      echo "BUILD FAILED: $label"; fail=1; continue
    fi
    dir=target/$([ -n "$profile" ] && echo release || echo debug)
    rust_so=$dir/libarity_lib.so

    # ---- symbol parity (Phase D gate) ----
    if diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
            <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort) >/dev/null; then
      echo "symbols: identical ($(nm -D --defined-only "$rust_so" | wc -l) exported)"
    else
      echo "SYMBOL DIFF (C vs Rust):"; fail=1
      diff <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort) \
           <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort)
    fi
    if ldd -r "$rust_so" 2>&1 | grep -q "undefined symbol"; then
      echo "UNDEFINED SYMBOLS in $rust_so:"; fail=1
      ldd -r "$rust_so" 2>&1 | grep "undefined symbol"
    else
      echo "ldd -r: no undefined symbols"
    fi

    # ---- differential suites ----
    if cargo test $combo $profile --tests >target/run-all.log 2>&1; then
      grep -hE '^test result' target/run-all.log | sed 's/^/  /'
    else
      echo "TESTS FAILED: $label"; fail=1
      grep -hE '^test [a-z0-9_]+ \.\.\. FAILED|panicked at' target/run-all.log | head -20
    fi

    # ---- cross-profile: run this profile's tests against the OTHER .so ----
    other=target/$([ -n "$profile" ] && echo debug || echo release)/libarity_lib.so
    if [ -f "$other" ]; then
      if RUST_LIB_PATH=$(readlink -f "$other") \
         cargo test $combo $profile --tests >target/run-all-x.log 2>&1; then
        echo "cross-profile (.so = $other): ok"
      else
        echo "CROSS-PROFILE TESTS FAILED: $label with $other"; fail=1
        grep -hE '^test [a-z0-9_]+ \.\.\. FAILED|panicked at' target/run-all-x.log | head -20
      fi
    fi
  done
done

# ------------------------------------------------------------- 5. mutation check
if [ "${SKIP_MUTATION:-0}" != "1" ]; then
  for profile in "" "--release"; do
    note "mutation check (test power), profile='${profile:-dev}'"
    if PROFILE=$profile ./mutation_check.sh 2>&1 | tail -4; then :; else
      echo "MUTATION CHECK FAILED (profile ${profile:-dev})"; fail=1
    fi
  done
fi

note "SUMMARY"
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (see above)"
fi
exit $fail
