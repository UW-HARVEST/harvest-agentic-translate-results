#!/usr/bin/env bash
# Build the C .so, then for EVERY feature combination: cargo check, rebuild the
# Rust cdylib (cargo test does NOT rebuild a cdylib-only lib target!), and run
# the differential test suite.
set -u
cd "$(dirname "$0")/.." || exit 1

echo "=== building the C shared object ==="
(
  cd c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "${TMPDIR:-/tmp}/cmake.log" 2>&1 \
    && cmake --build . >> "${TMPDIR:-/tmp}/cmake.log" 2>&1
) || { echo "C build FAILED"; tail -20 "${TMPDIR:-/tmp}/cmake.log"; exit 1; }
ls -l c_src/build/*.so

rc=0
while IFS= read -r combo; do
    label="${combo:-<none (default)>}"
    echo
    echo "############ feature combination: $label ############"
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")

    echo "--- cargo check ---"
    timeout 600 cargo check "${args[@]}" --all-targets || { echo "CHECK FAILED: $label"; rc=1; continue; }
    echo "--- cargo build (cdylib) ---"
    timeout 600 cargo build "${args[@]}" || { echo "BUILD FAILED: $label"; rc=1; continue; }
    echo "--- cargo test ---"
    timeout 600 cargo test "${args[@]}" 2>&1 | tail -60 || rc=1
    if [ "${PIPESTATUS[0]:-0}" != 0 ]; then rc=1; fi
done < <(./scripts/feature_combos.sh)

echo
if [ "$rc" -eq 0 ]; then echo "ALL FEATURE COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$rc"
