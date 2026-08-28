#!/usr/bin/env bash
# Enumerates every valid Cargo feature combination and runs `cargo check` plus
# the full differential test suite against the C shared library for each one.
#
# Usage: translation/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
TIMEOUT=600
status=0

# ---------------------------------------------------------------------------
# 1. Build the C reference library (the CMakeLists has no build-time options,
#    so there is exactly one C configuration).
# ---------------------------------------------------------------------------
echo "== building C reference library =="
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build .
) >/tmp/cmake-build.log 2>&1 || { echo "C build FAILED (see /tmp/cmake-build.log)"; exit 1; }
ls -1 "$ROOT"/c_src/build/*.so

# ---------------------------------------------------------------------------
# 2. Enumerate feature combinations: read the [features] table from Cargo.toml
#    and take its power set. With no [features] table the only configuration is
#    the empty one.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml | grep -v '^default$'
)

combos=("")
for f in "${FEATURES[@]}"; do
  for existing in "${combos[@]}"; do
    if [ -z "$existing" ]; then combos+=("$f"); else combos+=("$existing,$f"); fi
  done
done

echo
echo "== features declared: ${#FEATURES[@]} (${FEATURES[*]-none}) =="
echo "== feature combinations to verify: ${#combos[@]} =="

# ---------------------------------------------------------------------------
# 3. check + test each combination.
# ---------------------------------------------------------------------------
for combo in "${combos[@]}"; do
  if [ -z "$combo" ]; then
    args=(--no-default-features)
    label="<no features>"
  else
    args=(--no-default-features --features "$combo")
    label="$combo"
  fi

  echo
  echo "---------- combination: $label ----------"

  if ! timeout "$TIMEOUT" cargo check "${args[@]}" >/tmp/check.log 2>&1; then
    echo "  cargo check FAILED"; tail -30 /tmp/check.log; status=1; continue
  fi
  echo "  cargo check ok"

  # Build the cdylib explicitly: `cargo test` alone does not produce a
  # cdylib-only lib target (the harness can bootstrap it, but the symbol
  # comparison below wants it in the usual place).
  if ! timeout "$TIMEOUT" cargo build "${args[@]}" >/tmp/build.log 2>&1; then
    echo "  cargo build FAILED"; tail -30 /tmp/build.log; status=1; continue
  fi
  echo "  cargo build ok"

  if ! timeout "$TIMEOUT" cargo test "${args[@]}" >/tmp/test.log 2>&1; then
    echo "  cargo test FAILED"; tail -40 /tmp/test.log; status=1; continue
  fi
  grep -E '^test result:' /tmp/test.log | sed 's/^/  debug  /'

  # Repeat against the optimised profile: different codegen, same expected
  # results.
  if ! timeout "$TIMEOUT" cargo test --release "${args[@]}" >/tmp/test-release.log 2>&1; then
    echo "  cargo test --release FAILED"; tail -40 /tmp/test-release.log; status=1; continue
  fi
  grep -E '^test result:' /tmp/test-release.log | sed 's/^/  release /'

  # Symbol parity, printed for the record (also asserted by tests/symbols.rs).
  c_so=$(ls -1 "$ROOT"/c_src/build/*.so | head -1)
  rust_so=target/debug/libmatrixsum_lib.so
  missing=$(comm -23 \
    <(nm -D --defined-only "$c_so"    | awk 'NF>=3 {print $NF}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk 'NF>=3 {print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  MISSING EXPORTS: $missing"; status=1
  else
    echo "  symbol parity ok"
  fi
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL COMBINATIONS PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$status"
