#!/usr/bin/env bash
# Full differential verification run.
#
# IMPORTANT: `cargo test` does NOT rebuild a cdylib-only lib target, so the
# `cargo build` step below is mandatory and must come first.  The test harness
# also refuses to run against a `.so` that is older than `src/lib.rs`.
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"

echo "=== 1. building the C shared library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . 2>&1 | tail -3
) || exit 1
ls -l "$ROOT"/c_src/build/*.so

# The crate declares no [features] table, so the only distinct configurations
# are the default one and --no-default-features.  Enumerate them mechanically
# anyway, so that adding a feature later is picked up automatically.
FEATURE_SETS=()
if grep -q '^\[features\]' Cargo.toml; then
  FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' Cargo.toml \
          | grep -v '^default$' | tr -d '"' | sort -u)
  FEATURE_SETS+=("")                       # default
  FEATURE_SETS+=("--no-default-features")  # nothing
  for f in $FEATS; do
    FEATURE_SETS+=("--no-default-features --features $f")
  done
  if [ -n "$FEATS" ]; then
    ALL=$(echo "$FEATS" | paste -sd, -)
    FEATURE_SETS+=("--no-default-features --features $ALL")
    FEATURE_SETS+=("--all-features")
  fi
else
  FEATURE_SETS+=("")
  FEATURE_SETS+=("--no-default-features")
  FEATURE_SETS+=("--all-features")
fi

FAIL=0
for FS in "${FEATURE_SETS[@]}"; do
  label="${FS:-<default>}"
  echo
  echo "############################################################"
  echo "### feature set: $label"
  echo "############################################################"

  echo "--- cargo check"
  # shellcheck disable=SC2086
  timeout 600 cargo check --release $FS 2>&1 | tail -3 || FAIL=1

  echo "--- cargo build --release (rebuilds the cdylib the tests dlopen)"
  # shellcheck disable=SC2086
  timeout 600 cargo build --release $FS 2>&1 | tail -3 || FAIL=1

  echo "--- symbol diff (C .so vs Rust .so)"
  diff <(nm -D --defined-only "$ROOT"/c_src/build/*.so | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libarr_del_lib.so \
           | awk '$(NF-1)=="T"{print $NF}' | sort) \
    && echo "    symbol diff: EMPTY (ok)" \
    || { echo "    symbol diff: NON-EMPTY (FAIL)"; FAIL=1; }

  echo "--- cargo test --release"
  # shellcheck disable=SC2086
  timeout 600 cargo test --release $FS 2>&1 | grep -E "^(running|test |test result|error|warning)" | tail -200 || FAIL=1
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL FEATURE SETS PASSED"
else
  echo "FAILURES DETECTED"
fi
exit "$FAIL"
