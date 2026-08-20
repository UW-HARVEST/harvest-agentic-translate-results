#!/usr/bin/env bash
# Full differential verification: builds the C reference .so, then runs
# `cargo check` + the whole differential suite for EVERY valid feature
# combination of the crate.
#
# Feature combinations are enumerated mechanically from Cargo.toml rather than
# hard-coded, so this keeps working if a [features] table is ever added.
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$PWD"
FAIL=0

step() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
# 1. C reference shared library
# ---------------------------------------------------------------------------
step "building the C reference .so"
mkdir -p c_src/build
( cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l c_src/build/libdriver.so || exit 1

# ---------------------------------------------------------------------------
# 2. Enumerate every valid feature combination (powerset of [features])
# ---------------------------------------------------------------------------
step "enumerating feature combinations from Cargo.toml"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
        split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
        if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
N=${#FEATURES[@]}
echo "optional features found: ${N}${FEATURES[*]+ (${FEATURES[*]})}"

COMBOS=()
if (( N == 0 )); then
  COMBOS+=("")                      # only the empty set exists
else
  for (( mask = 0; mask < (1 << N); mask++ )); do
    combo=""
    for (( i = 0; i < N; i++ )); do
      if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 3. cargo check + build + full test suite for each combination
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  step "combo: $label"

  if ! timeout 600 cargo check --offline --no-default-features \
        ${combo:+--features "$combo"} --all-targets 2>&1 | tail -n 5; then
    echo "cargo check FAILED for $label"; FAIL=1; continue
  fi

  # The tests dlopen target/debug/libdriver.so, so it must be freshly built
  # for this exact feature set.
  if ! timeout 600 cargo build --offline --no-default-features \
        ${combo:+--features "$combo"} 2>&1 | tail -n 3; then
    echo "cargo build FAILED for $label"; FAIL=1; continue
  fi

  if ! timeout 600 cargo test --offline --no-default-features \
        ${combo:+--features "$combo"} 2>&1 | tail -n 25; then
    echo "cargo test FAILED for $label"; FAIL=1
  fi

  # Re-run single-threaded too: fd-1 capture is a process-global operation and
  # both scheduling regimes must give the same verdict.
  if ! timeout 600 cargo test --offline --no-default-features \
        ${combo:+--features "$combo"} -- --test-threads=1 2>&1 | tail -n 8; then
    echo "cargo test (single-threaded) FAILED for $label"; FAIL=1
  fi
done

# ---------------------------------------------------------------------------
# 4. Symbol parity, printed for the record
# ---------------------------------------------------------------------------
step "symbol parity (nm -D --defined-only)"
echo "--- C ---";    nm -D --defined-only "$ROOT/c_src/build/libdriver.so"
echo "--- Rust ---"; nm -D --defined-only "$ROOT/target/debug/libdriver.so"
missing=$(comm -23 \
  <(nm -D --defined-only "$ROOT/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u) \
  <(nm -D --defined-only "$ROOT/target/debug/libdriver.so" | awk '{print $NF}' | sort -u))
if [[ -n "$missing" ]]; then
  echo "MISSING FROM RUST .so:"; echo "$missing"; FAIL=1
else
  echo "symbol diff is EMPTY (0 missing)"
fi

step "result"
if (( FAIL )); then echo "VERIFICATION FAILED"; exit 1; fi
echo "VERIFICATION PASSED for all ${#COMBOS[@]} feature combination(s)"
