#!/usr/bin/env bash
# Differential test driver.
#
#   1. builds the C ground-truth shared library with CMake
#   2. enumerates every Cargo feature combination from Cargo.toml
#   3. for each combination: rebuilds the Rust cdylib, then runs the whole
#      differential suite against that freshly built .so
#   4. repeats for both the release and the debug profile (different codegen /
#      panic strategy => different code paths)
#
# `cargo test` does NOT build the cdylib artifact, so the explicit
# `cargo build` before each `cargo test` is mandatory; the harness also asserts
# the .so is newer than src/lib.rs so a stale object can never pass silently.

set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
CARGO_FLAGS="--offline"

echo "=== [1/4] building C ground truth ==="
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
echo "C  .so: $C_SO"

echo "=== [2/4] enumerating feature combinations ==="
# Every feature declared in [features] (excluding the implicit "default").
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /^[A-Za-z0-9_-]+[ ]*=/ {
    split($0, a, "="); gsub(/[ \t]/, "", a[1]);
    if (a[1] != "default") print a[1]
  }' "$CRATE_DIR/Cargo.toml")

if [ -z "$FEATURES" ]; then
  echo "no [features] declared -> the only configurations are:"
  COMBOS=("--offline" "--offline --no-default-features" "--offline --all-features")
else
  echo "features: $FEATURES"
  # power set of the declared features
  mapfile -t FLIST <<<"$FEATURES"
  n=${#FLIST[@]}
  COMBOS=("--offline --no-default-features" "--offline" "--offline --all-features")
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel="$sel,${FLIST[$i]}"; fi
    done
    COMBOS+=("--offline --no-default-features --features ${sel#,}")
  done
fi
printf '  %s\n' "${COMBOS[@]}"

echo "=== [3/4] cargo check across every combination ==="
cd "$CRATE_DIR"
for combo in "${COMBOS[@]}"; do
  echo "--- cargo check $combo"
  # shellcheck disable=SC2086
  cargo check $combo --all-targets 2>&1 | tail -2
done

echo "=== [4/4] differential suite across every combination x profile ==="
fail=0
for combo in "${COMBOS[@]}"; do
  for profile in release debug; do
    prof_flag=""
    [ "$profile" = release ] && prof_flag="--release"
    echo
    echo "########## combo='$combo' profile=$profile ##########"
    # shellcheck disable=SC2086
    cargo build $combo $prof_flag 2>&1 | tail -1
    so="$CRATE_DIR/target/$profile/libconfusion_lib.so"
    [ -f "$so" ] || { echo "MISSING $so"; fail=1; continue; }
    # shellcheck disable=SC2086
    if DIFFTEST_RUST_SO="$so" cargo test $combo -- --test-threads=1 2>&1 \
        | grep -E 'test result|FAILED|panicked'; then :; fi
    # shellcheck disable=SC2086
    if ! DIFFTEST_RUST_SO="$so" cargo test $combo -- --test-threads=1 >/dev/null 2>&1; then
      echo "!!! FAILURES for combo='$combo' profile=$profile"
      fail=1
    fi
  done
done

echo
if [ "$fail" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$fail"
