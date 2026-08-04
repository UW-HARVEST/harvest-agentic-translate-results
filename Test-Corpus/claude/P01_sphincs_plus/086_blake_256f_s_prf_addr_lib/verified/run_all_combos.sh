#!/usr/bin/env bash
# Runs cargo test for every (hash, thash, secpar) combination, after
# rebuilding the C lib for that combination. Stops on first failure.
#
# Usage: ./run_all_combos.sh [hash_filter] [secpar_filter]

set -uo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

HASH_FILTER="${1:-}"
SECPAR_FILTER="${2:-}"

HASHES=("blake" "haraka" "sha2" "shake")
THASHES=("simple" "robust")
SECPARS=("128s" "128f" "192s" "192f" "256s" "256f")

if [ -n "$HASH_FILTER" ]; then HASHES=("$HASH_FILTER"); fi
if [ -n "$SECPAR_FILTER" ]; then SECPARS=("$SECPAR_FILTER"); fi

failed=0
for hash in "${HASHES[@]}"; do
  for thash in "${THASHES[@]}"; do
    for secpar in "${SECPARS[@]}"; do
      combo="$hash,$thash,$secpar"
      echo "===== $combo ====="

      # Rebuild C lib for this combo. The C side depends only on hash and
      # secpar (and thash compiles different thash_*.c). Use shake256 name
      # for shake backend folder (matches CMake var).
      hash_dir="$hash"
      hash_cmake="$hash"
      cmake_thash="$thash"
      cmake_secpar="$secpar"

      rm -rf c_src/build
      mkdir -p c_src/build
      cmake -S c_src -B c_src/build \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DHASH_BACKEND="$hash_cmake" \
        -DSECPAR="$cmake_secpar" \
        -DTHASH="$cmake_thash" >/tmp/cmake.log 2>&1
      if [ $? -ne 0 ]; then
        echo "  FAIL: cmake config"
        cat /tmp/cmake.log
        failed=$((failed+1)); continue
      fi
      cmake --build c_src/build >/tmp/cbuild.log 2>&1
      if [ $? -ne 0 ]; then
        echo "  FAIL: C build"
        tail -30 /tmp/cbuild.log
        failed=$((failed+1)); continue
      fi

      # Build Rust
      cargo build --release --no-default-features --features "$combo" >/tmp/rbuild.log 2>&1
      if [ $? -ne 0 ]; then
        echo "  FAIL: Rust build"
        tail -30 /tmp/rbuild.log
        failed=$((failed+1)); continue
      fi

      # Run tests
      timeout 300 cargo test --release --no-default-features --features "$combo" \
        --tests >/tmp/rtest.log 2>&1
      rc=$?
      if [ $rc -ne 0 ]; then
        echo "  FAIL: cargo test (exit=$rc)"
        tail -50 /tmp/rtest.log
        failed=$((failed+1))
      else
        echo "  OK"
      fi
    done
  done
done

echo "================="
if [ $failed -gt 0 ]; then
  echo "$failed combinations FAILED"
  exit 1
else
  echo "ALL PASSED"
fi
