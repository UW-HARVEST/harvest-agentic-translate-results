#!/usr/bin/env bash
# Differential test runner: C .so vs Rust .so.
#
# IMPORTANT: `cargo test` does NOT rebuild a `cdylib` target, so the Rust `.so`
# MUST be rebuilt explicitly before the tests run — otherwise the tests load a
# stale library and pass vacuously. This script does that, and also rebuilds the
# C `.so`, then runs the suite under every feature combination.

set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
CARGO_FLAGS="${CARGO_FLAGS:---offline}"
PROFILE="${PROFILE:---release}"

echo "=== 1. Building the C shared library ==="
mkdir -p "$ROOT/c_src/build"
cmake -S "$ROOT/c_src" -B "$ROOT/c_src/build" -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
cmake --build "$ROOT/c_src/build"
C_SO="$(find "$ROOT/c_src/build" -name '*.so' | sort | head -1)"
echo "C  .so: $C_SO"

echo
echo "=== 2. Enumerating feature combinations ==="
# Mechanically extract the declared features (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/ {inf=0}
  inf && /^[a-zA-Z0-9_-]+[[:space:]]*=/ {
    split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' "$CRATE_DIR/Cargo.toml")

# Build the list of configurations to test: default, no-default, then every
# subset of the declared features. With no [features] table this collapses to
# {default, no-default-features}.
declare -a COMBOS=("" "--no-default-features")
if [[ -n "$FEATURES" ]]; then
  mapfile -t FEAT_ARR <<<"$FEATURES"
  n=${#FEAT_ARR[@]}
  echo "Declared features: ${FEAT_ARR[*]}"
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then
        combo="${combo:+$combo,}${FEAT_ARR[$i]}"
      fi
    done
    COMBOS+=("--no-default-features --features $combo")
    COMBOS+=("--features $combo")
  done
else
  echo "No [features] table in Cargo.toml -> only the default configuration exists."
fi
echo "Configurations to test: ${#COMBOS[@]}"

FAILED=0
for combo in "${COMBOS[@]}"; do
  echo
  echo "==============================================================="
  echo "=== Configuration: ${combo:-<default>}"
  echo "==============================================================="

  # Rebuild the cdylib for THIS configuration before testing it.
  # shellcheck disable=SC2086
  cargo build --manifest-path "$CRATE_DIR/Cargo.toml" $CARGO_FLAGS $PROFILE $combo
  RUST_SO=$(find "$CRATE_DIR/target" -name 'libencode_quant_lib.so' -newermt '-1 day' | head -1)
  echo "Rust .so: ${RUST_SO:-<not found>}"

  # shellcheck disable=SC2086
  if cargo test --manifest-path "$CRATE_DIR/Cargo.toml" $CARGO_FLAGS $PROFILE $combo -- --test-threads="${TEST_THREADS:-4}"; then
    echo "RESULT: PASS (${combo:-<default>})"
  else
    echo "RESULT: FAIL (${combo:-<default>})"
    FAILED=1
  fi
done

echo
echo "=== 3. Symbol parity ==="
RUST_SO="$CRATE_DIR/target/release/libencode_quant_lib.so"
[[ -f "$RUST_SO" ]] || RUST_SO="$CRATE_DIR/target/debug/libencode_quant_lib.so"
diff <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort) \
     <(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort) \
  && echo "Symbol diff EMPTY: the Rust .so exports every C symbol." \
  || { echo "SYMBOL MISMATCH (see diff above)"; FAILED=1; }

echo
if ((FAILED)); then
  echo "OVERALL: FAILED"
  exit 1
fi
echo "OVERALL: ALL CONFIGURATIONS PASSED"
