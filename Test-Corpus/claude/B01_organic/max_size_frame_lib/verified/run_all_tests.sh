#!/usr/bin/env bash
# Phase D driver: run the whole differential suite for EVERY valid feature
# combination (and for both the dev and release profiles), then print the
# nm -D symbol diff between the C and Rust shared objects.
set -uo pipefail

cd "$(dirname "$0")"

CARGO_OFFLINE=${CARGO_OFFLINE:---offline}

echo "##############################################################"
echo "# 0. Build the C shared library (ground truth)"
echo "##############################################################"
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
# Pin the exact name CMake produces (project name = parent dir of c_src), so a
# stray .so in c_src/build can never be mistaken for the ground truth.
C_SO="$PWD/c_src/build/lib$(basename "$PWD").so"
[ -f "$C_SO" ] || { echo "expected C library $C_SO not found"; exit 1; }
echo "C .so: $C_SO"
echo

# ---- Enumerate feature combinations (power set of the [features] table) ----
mapfile -t FEATURES < <(
  awk '
    /^[[:space:]]*\[/ { in_f = ($0 ~ /^[[:space:]]*\[features\][[:space:]]*$/); next }
    in_f && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ {
      sub(/[[:space:]]*=.*/, ""); gsub(/[[:space:]]/, "");
      if ($0 != "default") print
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}

COMBOS=()
if [ "$N" -eq 0 ]; then
  COMBOS+=("")            # no [features] table => empty set is the only config
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      (((mask >> i) & 1)) && combo="${combo:+$combo,}${FEATURES[i]}"
    done
    COMBOS+=("$combo")
  done
fi

echo "##############################################################"
echo "# 1. Feature combinations discovered: ${#COMBOS[@]} (features: ${FEATURES[*]:-none})"
echo "##############################################################"
echo

rc=0
run_suite() {
  local label="$1"; shift
  local feature_args="$1"; shift
  local profile_args="$1"; shift

  echo "=============================================================="
  echo ">>> $label"
  echo ">>>   cargo test $feature_args $profile_args"
  echo "=============================================================="
  # DIFF_TEST_FEATURE_ARGS tells the harness which features to use when it
  # rebuilds the cdylib under test, so the .so always matches this combination.
  if DIFF_TEST_FEATURE_ARGS="$feature_args" \
       timeout 600 cargo test $CARGO_OFFLINE $feature_args $profile_args 2>&1 \
       | grep -E '^(test result|running|error|warning: unused|thread)|FAILED|DIVERGENCE'; then
    :
  fi
  # Re-run capturing status (grep above eats the exit code).
  if DIFF_TEST_FEATURE_ARGS="$feature_args" \
       timeout 600 cargo test $CARGO_OFFLINE $feature_args $profile_args >/dev/null 2>&1; then
    echo "RESULT: PASS  [$label]"
  else
    echo "RESULT: FAIL  [$label]"
    rc=1
  fi
  echo
}

for combo in "${COMBOS[@]}"; do
  # An empty combo must NOT emit a dangling `--features` (cargo rejects it).
  if [ -z "$combo" ]; then
    fargs="--no-default-features"
  else
    fargs="--no-default-features --features $combo"
  fi
  run_suite "features='${combo:-<empty>}' dev"     "$fargs" ""
  run_suite "features='${combo:-<empty>}' release" "$fargs" "--release"
done

run_suite "default features, dev" "" ""
run_suite "default features, release" "" "--release"
run_suite "all features, dev" "--all-features" ""
run_suite "all features, release" "--all-features" "--release"

echo "##############################################################"
echo "# 1b. Exhaustive 2^32 per-axis sweeps"
echo "##############################################################"
if [ "${RUN_EXHAUSTIVE:-0}" = "1" ]; then
  echo "Running the full 2^32 axis sweeps in release mode (~4 min)..."
  if timeout 600 cargo test $CARGO_OFFLINE --release --test exhaustive_axis \
       -- --ignored --nocapture --test-threads=4 2>&1 | tail -30; then
    echo "RESULT: PASS  [exhaustive 2^32 sweeps]"
  else
    echo "RESULT: FAIL  [exhaustive 2^32 sweeps]"
    rc=1
  fi
else
  echo "SKIPPED (set RUN_EXHAUSTIVE=1 to enable; takes ~4 minutes)."
  echo "  RUN_EXHAUSTIVE=1 ./run_all_tests.sh"
fi
echo

echo "##############################################################"
echo "# 2. nm -D symbol parity (C .so vs Rust .so)"
echo "##############################################################"
RUST_SO=$(find "$PWD/target/diff-so" -name 'libmax_size_frame_lib.so' | head -1)
[ -z "$RUST_SO" ] && RUST_SO="$PWD/target/debug/libmax_size_frame_lib.so"
echo "Rust .so: $RUST_SO"
echo
echo "--- C exports ---";    nm -D --defined-only "$C_SO"
echo "--- Rust exports ---"; nm -D --defined-only "$RUST_SO" | grep -vE ' (__|_ITM|_init|_fini)' || true
echo
echo "--- symbols in C but MISSING from Rust (must be empty) ---"
diff <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
     <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u) \
     | grep '^<' && { echo "SYMBOL PARITY FAILED"; rc=1; } || echo "(empty) SYMBOL PARITY OK"

echo
echo "##############################################################"
if [ "$rc" -eq 0 ]; then
  echo "# ALL CONFIGURATIONS PASSED"
else
  echo "# FAILURES PRESENT"
fi
echo "##############################################################"
exit "$rc"
