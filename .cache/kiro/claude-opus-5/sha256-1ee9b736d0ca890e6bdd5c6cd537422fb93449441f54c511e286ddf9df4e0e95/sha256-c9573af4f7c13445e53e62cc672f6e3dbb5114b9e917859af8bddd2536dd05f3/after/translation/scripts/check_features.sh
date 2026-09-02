#!/usr/bin/env bash
# Phase D: run the full differential suite under EVERY feature combination.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so this
# stays correct if features are added later. The powerset is enumerated; if the
# crate has no features, the single default configuration is run.
#
# Usage:  ./scripts/check_features.sh        (from translation/)
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
ROOT="$(cd .. && pwd)"
TIMEOUT=${TIMEOUT:-600}

# --- 1. Make sure the C ground truth exists -------------------------------
C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' 2>/dev/null | sort | tail -1)
if [[ -z "$C_SO" ]]; then
  echo "Building the C ground truth..."
  ( cd "$ROOT/c_src" && mkdir -p build && cd build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
  C_SO=$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | sort | tail -1)
fi
echo "C ground truth: $C_SO"

# --- 2. Extract feature names from the [features] table -------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

echo "features declared: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# --- 3. Build the combination list (powerset, plus the default build) -----
COMBOS=("<default>")
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=0; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

# --- 4. For each combination: rebuild the cdylib, then run every phase ----
FAILED=0
for combo in "${COMBOS[@]}"; do
  if [[ "$combo" == "<default>" ]]; then
    FLAGS=()
    label="default features"
  else
    FLAGS=(--no-default-features --features "$combo")
    label="--no-default-features --features '${combo:-<none>}'"
  fi

  echo
  echo "==================================================================="
  echo "CONFIG: $label"
  echo "==================================================================="

  # The cdylib MUST be rebuilt first: `cargo test` does not refresh a
  # crate-type=["cdylib"] artifact, and the suite dlopens it from disk.
  if ! timeout "$TIMEOUT" cargo build --release "${FLAGS[@]}" >/tmp/fc_build.log 2>&1; then
    echo "  BUILD FAILED"; tail -20 /tmp/fc_build.log; FAILED=1; continue
  fi
  touch target/release/libcleanup_lib.so   # defeat the harness staleness guard
  echo "  build: ok"

  if ! timeout "$TIMEOUT" cargo check --release --all-targets "${FLAGS[@]}" \
        >/tmp/fc_check.log 2>&1; then
    echo "  CHECK FAILED"; tail -20 /tmp/fc_check.log; FAILED=1; continue
  fi
  echo "  check: ok"

  for phase in phase_d_symbols phase_b_valid phase_c_errors; do
    if timeout "$TIMEOUT" cargo test --release "${FLAGS[@]}" --test "$phase" \
         -- --test-threads=1 >/tmp/fc_$phase.log 2>&1; then
      echo "  $phase: $(grep -o 'test result: ok\. [0-9]* passed' /tmp/fc_$phase.log | tail -1)"
    else
      echo "  $phase: FAILED"
      grep -E 'DIVERGENCE|panicked|assertion|test result' /tmp/fc_$phase.log | head -20
      FAILED=1
    fi
  done
done

echo
if (( FAILED )); then
  echo "RESULT: at least one configuration FAILED"
  exit 1
fi
echo "RESULT: all ${#COMBOS[@]} configuration(s) PASSED"
