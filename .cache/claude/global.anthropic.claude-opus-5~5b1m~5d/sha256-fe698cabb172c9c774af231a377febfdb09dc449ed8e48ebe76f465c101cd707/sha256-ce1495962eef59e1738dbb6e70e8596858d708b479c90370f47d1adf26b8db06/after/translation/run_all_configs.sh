#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every feature
# combination, plus the symbol diff and the exhaustive 2^32 sweep.
#
# Usage:  ./run_all_configs.sh [--exhaustive]
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
OFFLINE="--offline"   # the sandbox has no crates.io egress; libloading is cached
LOG="$(mktemp "${TMPDIR:-.}/rev16-test.XXXXXX")"
trap 'rm -f "$LOG"' EXIT
FAILED=0

say() { printf '\n=========== %s ===========\n' "$*"; }

# --- 0. Build the C shared library (ground truth) -------------------------
say "building C ground-truth shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name 'lib*.so' | head -1)"
echo "C  .so: $C_SO"

# --- 1. Enumerate feature combinations from Cargo.toml --------------------
# Everything between a [features] header and the next [section] header.
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[ ]*=/ {sub(/[ ]*=.*/,""); print}
' Cargo.toml)

if [ -z "$FEATURES" ]; then
  echo "Cargo.toml declares no [features]; the complete combination set is:"
  echo "  (default) / --no-default-features / --all-features"
  COMBOS=("" "--no-default-features" "--all-features")
else
  echo "declared features: $FEATURES"
  COMBOS=("" "--no-default-features" "--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

# --- 2. Sanity-check that no code is feature-gated ------------------------
say "checking for feature gates in src/"
n=$(grep -c 'cfg(feature' src/lib.rs || true)
echo "occurrences of cfg(feature) in src/lib.rs: $n"

# --- 3. Run the suite under each combination -----------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-(default)}"
  say "cargo test $label"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $OFFLINE $combo >/dev/null 2>&1
  # shellcheck disable=SC2086
  if timeout 600 cargo test $OFFLINE $combo 2>&1 | tee "$LOG" | grep -E 'test result'; then
    if grep -qE '[1-9][0-9]* failed' "$LOG"; then
      echo ">>> FAILURES under $label"; FAILED=1
    else
      echo ">>> PASS under $label"
    fi
  else
    echo ">>> ERROR running $label"; FAILED=1
  fi
done

# --- 4. Symbol diff, printed for the record ------------------------------
say "nm -D symbol diff (C vs Rust)"
diff <(nm -D --defined-only "$C_SO"        | awk '{print $NF}' | sort) \
     <(nm -D --defined-only target/release/librev16_lib.so | awk '{print $NF}' | sort) \
  && echo "symbol diff EMPTY"

# --- 5. Optional exhaustive sweep ----------------------------------------
if [ "${1:-}" = "--exhaustive" ]; then
  say "exhaustive 2^32 differential sweep"
  timeout 600 cargo test $OFFLINE --test valid_paths -- --ignored --nocapture 2>&1 | tail -6 \
    || FAILED=1
fi

say "OVERALL: $([ $FAILED -eq 0 ] && echo ALL GREEN || echo FAILURES)"
exit $FAILED
