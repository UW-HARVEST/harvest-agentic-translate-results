#!/usr/bin/env bash
# Full verification sweep: build the C .so, then for every Cargo feature
# combination build the Rust cdylib and run the differential suite against it.
#
# IMPORTANT: `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact
# (integration tests never link it), so the cdylib must be built explicitly
# BEFORE the tests run.  The harness also refuses to run against a `.so` older
# than `src/lib.rs`, so a missing build step fails loudly instead of silently
# testing a stale library.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
CARGO_FLAGS="${CARGO_FLAGS:---offline}"

echo "### 1. Building the C shared library"
mkdir -p "$ROOT/c_src/build"
(cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)
C_SO="$ROOT/c_src/build/libStaticLoop.so"
echo "    -> $C_SO"

echo
echo "### 2. Enumerating feature combinations from Cargo.toml"
# Every declared feature (empty output => the crate has no [features] table).
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/            {inside=0}
  inside && /=/    {split($0, a, "="); gsub(/[ \t"]/, "", a[1]); if (a[1] != "") print a[1]}
' "$HERE/Cargo.toml" || true)

COMBOS=()
if [[ -z "$FEATURES" ]]; then
  echo "    no [features] table -> the only configurations are the default build"
  echo "    and --no-default-features (identical for this crate)"
  COMBOS+=("default:")
  COMBOS+=("no-default:--no-default-features")
else
  echo "    declared features: $FEATURES"
  # Power set of the declared features, plus the plain default build.
  FEAT_ARR=($FEATURES)
  n=${#FEAT_ARR[@]}
  COMBOS+=("default:")
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then combo+="${FEAT_ARR[b]},"; fi
    done
    combo="${combo%,}"
    if [[ -z "$combo" ]]; then
      COMBOS+=("no-default:--no-default-features")
    else
      COMBOS+=("no-default+$combo:--no-default-features --features $combo")
    fi
  done
fi

echo
echo "### 3. cargo check / build / test for every combination and profile"
FAILED=0
for entry in "${COMBOS[@]}"; do
  name="${entry%%:*}"
  flags="${entry#*:}"
  for profile in release debug; do
    prof_flag=""
    [[ "$profile" == "release" ]] && prof_flag="--release"
    echo
    echo "---- combo=[$name] profile=$profile flags=[$flags] ----"
    # shellcheck disable=SC2086
    if ! (cd "$HERE" \
          && cargo check $CARGO_FLAGS $prof_flag $flags 2>&1 | tail -3 \
          && cargo build $CARGO_FLAGS $prof_flag $flags 2>&1 | tail -2 \
          && STATICLOOP_C_SO="$C_SO" \
             STATICLOOP_RUST_SO="$HERE/target/$profile/libStaticLoop.so" \
             timeout 600 cargo test $CARGO_FLAGS $prof_flag $flags 2>&1 \
             | grep -E "^test result:|FAILED|^error"); then
      echo "!!! FAILED: combo=[$name] profile=$profile"
      FAILED=1
    fi
  done
done

echo
if [[ $FAILED -eq 0 ]]; then
  echo "### ALL FEATURE COMBINATIONS PASSED"
else
  echo "### SOME COMBINATIONS FAILED"
  exit 1
fi
