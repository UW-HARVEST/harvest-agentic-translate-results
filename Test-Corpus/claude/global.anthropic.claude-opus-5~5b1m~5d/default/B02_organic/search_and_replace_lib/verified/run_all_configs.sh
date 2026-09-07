#!/usr/bin/env bash
# Phase D: run the full differential suite under every build configuration.
#
# `cargo metadata` reports {} for this package's [features] table, so there is
# exactly ONE feature combination; --no-default-features and --all-features are
# run anyway to prove that. The two cargo PROFILES are genuinely different code,
# though: [profile.release] sets panic = "abort" while the dev profile unwinds,
# and dev has debug assertions / overflow checks on, so both are exercised.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"

echo "### building the C reference shared library"
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$ROOT/c_src/build/libdriver.so"

rc=0
for profile in dev release; do
  for feat in "" "--no-default-features" "--all-features"; do
    label="profile=$profile features=${feat:-<default>}"
    prof_flag=""; [ "$profile" = release ] && prof_flag="--release"
    out_dir="$HERE/target/$([ "$profile" = release ] && echo release || echo debug)"
    RUST_SO="$out_dir/libdriver.so"
    echo
    echo "=== $label ==="
    # `cargo test` alone does not produce the cdylib, so build it explicitly and
    # point the harness at exactly this configuration's .so (no silent fallback
    # to the other profile's artifact).
    rm -f "$RUST_SO"
    if ! timeout 300 cargo build --offline $prof_flag $feat >/dev/null 2>&1; then
      echo ">>> cdylib BUILD FAILED: $label"; rc=1; continue
    fi
    if ! timeout 900 env RUST_DRIVER_SO="$RUST_SO" C_DRIVER_SO="$C_SO" \
         cargo test --offline --no-fail-fast $prof_flag $feat -- --test-threads=1 2>&1 \
         | grep -E '^(test result|error|test .* FAILED|running )'; then
      echo ">>> FAILED: $label"; rc=1
    fi
    # Symbol parity for the .so produced by this configuration.
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u) \
      <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u))
    if [ -n "$missing" ]; then
      echo ">>> SYMBOL PARITY FAILED for $label; missing from Rust .so:"; echo "$missing"; rc=1
    else
      echo "symbol parity: OK (0 missing)"
    fi
  done
done

echo
echo "====================================================="
[ $rc -eq 0 ] && echo "ALL CONFIGURATIONS PASSED" || echo "SOME CONFIGURATIONS FAILED"
exit $rc
