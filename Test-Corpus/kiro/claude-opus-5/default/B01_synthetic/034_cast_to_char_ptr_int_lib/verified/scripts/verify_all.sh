#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build-time
# configuration: enumerate the feature powerset from Cargo.toml, `cargo check`
# each combination, then run the differential test suite for each one.
#
# Run from anywhere: scripts/verify_all.sh
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRATE="$(dirname "$HERE")"
ROOT="$(dirname "$CRATE")"
TIMEOUT=600
rc=0

# ---------------------------------------------------------------- C ground truth
if [[ ! -f "$ROOT/c_src/build/libdriver.so" ]]; then
  echo "== building C shared library =="
  mkdir -p "$ROOT/c_src/build"
  ( cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
fi
echo "C  .so: $ROOT/c_src/build/libdriver.so"

# ------------------------------------------------------- enumerate feature combos
# Names under [features] in Cargo.toml, excluding the "default" entry.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' "$CRATE/Cargo.toml"
)

n=${#FEATURES[@]}
echo "features declared in Cargo.toml: $n ${FEATURES[*]:-(none)}"

COMBOS=()
if (( n == 0 )); then
  # No features exist, so there is exactly one build configuration.
  COMBOS=("")
else
  for (( mask = 0; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi
echo "feature combinations to verify: ${#COMBOS[@]}"

cd "$CRATE" || exit 1

# ------------------------------------------------------------------ check + test
for combo in "${COMBOS[@]}"; do
  if [[ -z "$combo" ]]; then
    args=(--no-default-features)
    label="<no features>"
  else
    args=(--no-default-features --features "$combo")
    label="$combo"
  fi

  echo
  echo "=============================================================="
  echo "== configuration: $label"
  echo "=============================================================="

  echo "-- cargo check ${args[*]}"
  if ! timeout "$TIMEOUT" cargo check "${args[@]}" 2>&1 | tail -n 5; then
    echo "CHECK FAILED: $label"; rc=1; continue
  fi

  echo "-- cargo build --release ${args[*]} (exercises panic=abort profile)"
  if ! timeout "$TIMEOUT" cargo build --release "${args[@]}" 2>&1 | tail -n 3; then
    echo "RELEASE BUILD FAILED: $label"; rc=1; continue
  fi

  # The test harness rebuilds the cdylib itself; tell it which features to use.
  echo "-- cargo test ${args[*]} (debug cdylib)"
  log="/tmp/driver-test-debug.log"
  DRIVER_TEST_CARGO_ARGS="${args[*]}" \
    timeout "$TIMEOUT" cargo test "${args[@]}" >"$log" 2>&1
  status=$?
  grep -E "^test |test result" "$log"
  if (( status != 0 )); then
    echo "TESTS FAILED (debug cdylib): $label  [log: $log]"
    grep -E "panicked|mismatch|^error" "$log" | head -n 20
    rc=1
  fi

  # Same suite, but against the optimized release cdylib.
  echo "-- cargo test ${args[*]} (release cdylib via DRIVER_RUST_SO)"
  rlog="/tmp/driver-test-release.log"
  DRIVER_RUST_SO="$CRATE/target/release/libdriver.so" \
    timeout "$TIMEOUT" cargo test "${args[@]}" >"$rlog" 2>&1
  status=$?
  if (( status != 0 )); then
    echo "TESTS FAILED (release cdylib): $label  [log: $rlog]"
    grep -E "panicked|mismatch|^error" "$rlog" | head -n 20
    rc=1
  else
    echo "   release cdylib: ok"
  fi

  # Symbol parity for this configuration.
  echo "-- symbol comparison (nm -D)"
  diff <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort) \
       <(nm -D --defined-only "$CRATE/target/release/libdriver.so" | awk '{print $NF}' | sort) \
       > /tmp/driver-symdiff.txt
  if grep -q '^<' /tmp/driver-symdiff.txt; then
    echo "   MISSING in Rust .so:"; grep '^<' /tmp/driver-symdiff.txt
    rc=1
  else
    echo "   every C symbol is exported by the Rust .so"
  fi
done

echo
if (( rc == 0 )); then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "FAILURES DETECTED"
fi
exit $rc
