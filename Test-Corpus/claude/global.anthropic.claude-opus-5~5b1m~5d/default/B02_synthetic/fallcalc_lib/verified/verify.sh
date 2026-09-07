#!/usr/bin/env bash
# Full verification driver: builds the C .so and the Rust cdylib, then runs the
# whole differential suite across every feature combination x profile.
#
# Usage:  cd translation && ./verify.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAILED=0

note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. Build the C shared library (ground truth).
# ---------------------------------------------------------------------------
note "Building C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "C .so: $C_SO"

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml (power set).
# ---------------------------------------------------------------------------
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /^[A-Za-z0-9_-]+[ ]*=/{print $1}' \
        "$CRATE/Cargo.toml" | grep -v '^default$' | tr '\n' ' ')
echo "declared features: [${FEATS}]"

COMBOS=()
if [ -z "${FEATS// /}" ]; then
  # No [features] table -> default build and --no-default-features are the
  # same configuration; run both spellings anyway to prove it.
  COMBOS+=("")                       # default
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
else
  arr=($FEATS); n=${#arr[@]}
  COMBOS+=("")
  COMBOS+=("--no-default-features")
  COMBOS+=("--all-features")
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo+="${arr[$i]},"; fi
    done
    COMBOS+=("--no-default-features --features ${combo%,}")
  done
fi

# ---------------------------------------------------------------------------
# 3. For each combination x profile: rebuild the cdylib, pin it via
#    RUST_SO_PATH, and run the full suite against the C .so.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    if [ "$profile" = release ]; then PFLAG="--release"; else PFLAG=""; fi
    label="features=[${combo:-<default>}] profile=$profile"

    note "cargo check | $label"
    ( cd "$CRATE" && cargo check --offline $PFLAG $combo --all-targets ) \
      || { echo "CHECK FAILED: $label"; FAILED=1; continue; }

    note "cargo build (cdylib) | $label"
    ( cd "$CRATE" && cargo build --offline $PFLAG $combo ) \
      || { echo "BUILD FAILED: $label"; FAILED=1; continue; }

    SO="$CRATE/target/$profile/libfallcalc_lib.so"
    [ -f "$SO" ] || { echo "MISSING $SO"; FAILED=1; continue; }

    note "nm -D symbol diff | $label"
    nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/c_syms.txt"
    nm -D --defined-only "$SO"   | awk '{print $3}' | sort -u > "${TMPDIR:-/tmp}/r_syms.txt"
    MISSING="$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt")"
    if [ -n "$MISSING" ]; then
      echo "SYMBOL DIFF NOT EMPTY for $label:"; echo "$MISSING"; FAILED=1
    else
      echo "symbol diff empty ($(wc -l < "${TMPDIR:-/tmp}/c_syms.txt") C symbols all present)"
    fi

    note "cargo test | $label"
    ( cd "$CRATE" && RUST_SO_PATH="$SO" \
        timeout 600 cargo test --offline $PFLAG $combo -- --test-threads=4 ) \
      || { echo "TESTS FAILED: $label"; FAILED=1; }
  done
done

note "RESULT"
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
