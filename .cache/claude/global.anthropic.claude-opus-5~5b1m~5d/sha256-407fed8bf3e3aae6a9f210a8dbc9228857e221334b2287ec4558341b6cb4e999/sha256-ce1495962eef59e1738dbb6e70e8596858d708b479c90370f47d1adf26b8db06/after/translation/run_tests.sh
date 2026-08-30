#!/usr/bin/env bash
# Build BOTH shared objects, then run the differential test suite across every
# profile and feature combination.
#
# `cargo test` does not rebuild a cdylib-only target, so the explicit
# `cargo build` before each `cargo test` is REQUIRED — without it the tests load
# a stale .so and pass vacuously. tests/common/mod.rs also asserts freshness.
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
CARGO_FLAGS="--offline"

fail=0

# --- C shared library ------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  || { echo "FATAL: C build failed"; exit 1; }
echo "built: c_src/build/libpow.so"

cd "$CRATE_DIR"

# --- feature combinations --------------------------------------------------
# Cargo.toml declares no [features], so these are all the same build; they are
# run anyway so that adding a feature later is covered automatically.
FEATURE_SETS=(
  ""
  "--no-default-features"
  "--all-features"
)

for profile in "" "--release"; do
  for feats in "${FEATURE_SETS[@]}"; do
    label="cargo test ${profile:-<debug>} ${feats:-<default-features>}"
    echo
    echo "=============================================================="
    echo "$label"
    echo "=============================================================="
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $CARGO_FLAGS $profile $feats 2>&1 | tail -3; then
      echo "RESULT: BUILD FAILED -- $label"; fail=1; continue
    fi
    # shellcheck disable=SC2086
    out=$(timeout 600 cargo test $CARGO_FLAGS $profile $feats 2>&1)
    echo "$out" | grep -E '^(test result:|error|warning: unused)' | sort -u
    if echo "$out" | grep -qE '^test result: FAILED|^error'; then
      echo "RESULT: FAILED -- $label"
      echo "$out" | grep -E '\.\.\. FAILED' | head -20
      fail=1
    else
      echo "RESULT: PASS -- $label"
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
