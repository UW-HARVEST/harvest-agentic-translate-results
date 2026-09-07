#!/usr/bin/env bash
# Build the C .so, build the Rust cdylib, then run the differential tests.
#
# `cargo test` does NOT rebuild the cdylib target (the integration tests reach
# it through dlopen, not through a link edge), so the explicit `cargo build`
# below is load-bearing: without it the tests would dlopen a stale .so. The
# harness also asserts freshness, so a stale run fails loudly rather than
# passing silently.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

# --- C shared library -------------------------------------------------------
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null )
c_so=$(ls "$root"/c_src/build/*.so)
echo "C   .so: $c_so"

cd "$here"

# --- feature combinations ---------------------------------------------------
# Enumerated from Cargo.toml rather than hard-coded. This crate declares no
# [features], so the list collapses to the default (== --no-default-features).
feats=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {print $1}' Cargo.toml | tr -d '"' | grep -v '^default$' || true)

run_combo() {
    local label="$1"; shift
    for profile in "" "--release"; do
        local pname="debug"; [ -n "$profile" ] && pname="release"
        echo "=== $label / $pname ==="
        if ! timeout 600 cargo build $profile "$@" >/dev/null; then
            echo "BUILD FAILED: $label / $pname"; exit 1
        fi
        # The cdylib is built with panic = "abort", so a UB check (e.g. a
        # misaligned dereference) kills the test process without ever printing
        # "test result: FAILED". Never judge the run by grepping stdout — check
        # the exit status via PIPESTATUS.
        set +e
        timeout 600 cargo test $profile "$@" -- --test-threads="${TT:-8}" 2>&1 \
            | grep -E '^(test result|error|failures:|---- |thread .* panicked|misaligned)'
        local rc=${PIPESTATUS[0]}
        set -e
        if [ "$rc" -ne 0 ]; then
            echo "TESTS FAILED (exit $rc): $label / $pname"; exit 1
        fi
    done
}

run_combo "default features" 
run_combo "no-default-features" --no-default-features

if [ -n "$feats" ]; then
    for f in $feats; do
        run_combo "feature=$f" --no-default-features --features "$f"
    done
    # full cross-product of all declared features
    all=$(echo "$feats" | paste -sd, -)
    run_combo "features=$all" --no-default-features --features "$all"
else
    echo "=== no [features] declared in Cargo.toml: default == no-default-features ==="
fi

# --- symbol parity ----------------------------------------------------------
echo "=== nm -D symbol diff (C minus Rust must be empty) ==="
nm -D --defined-only "$c_so" | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only target/release/libcollided_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
if diff <(cat /tmp/c_syms.txt) <(comm -12 /tmp/c_syms.txt /tmp/r_syms.txt) >/dev/null; then
    echo "OK: all $(wc -l < /tmp/c_syms.txt) C symbols are exported by the Rust .so"
else
    echo "MISSING FROM RUST:"; comm -23 /tmp/c_syms.txt /tmp/r_syms.txt; exit 1
fi
