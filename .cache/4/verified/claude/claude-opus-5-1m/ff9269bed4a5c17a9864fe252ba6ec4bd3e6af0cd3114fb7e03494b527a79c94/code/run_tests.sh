#!/usr/bin/env bash
# Differential C-vs-Rust verification driver.
#
#   ./run_tests.sh            # build everything, then run every feature combination
#   ./run_tests.sh check      # only `cargo check` every feature combination
#
# `cargo test` does not rebuild a cdylib, so the Rust `.so` is (re)built
# explicitly before each test run; the harness additionally refuses to run
# against a stale artifact.

set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
fail=0

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination from Cargo.toml.
#    (No [features] table => the single combination is the empty set.)
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t"]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBOS=("")                      # empty feature set only
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "=== feature combinations (${#COMBOS[@]}) ==="
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '${c}'"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared library (default configuration; CMakeLists exposes no
#    options).
# ---------------------------------------------------------------------------
echo
echo "=== building C shared library ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# 3. cargo check / build / test for every combination.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  echo
  echo "############################################################"
  echo "# feature combination: $label"
  echo "############################################################"

  args=(--no-default-features --offline)
  [ -n "$combo" ] && args+=(--features "$combo")

  echo "--- cargo check ---"
  if ! timeout 600 cargo check "${args[@]}" --all-targets 2>&1 | tail -20; then
    echo "CHECK FAILED for $label"; fail=1; continue
  fi

  [ "${1:-}" = "check" ] && continue

  echo "--- cargo build (cdylib) ---"
  if ! timeout 600 cargo build "${args[@]}" 2>&1 | tail -5; then
    echo "BUILD FAILED for $label"; fail=1; continue
  fi
  ls -l target/debug/libdriver.so

  echo "--- nm -D symbol diff ---"
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort) \
    <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort))
  if [ -n "$missing" ]; then
    echo "SYMBOLS MISSING FROM RUST .so:"; echo "$missing"; fail=1
  else
    echo "symbol diff empty (0 missing)"
  fi

  echo "--- cargo test (Phase B + Phase C + Phase D) ---"
  if ! timeout 600 cargo test "${args[@]}" -- --test-threads=1 2>&1 | tail -60; then
    echo "TESTS FAILED for $label"; fail=1
  fi
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"
