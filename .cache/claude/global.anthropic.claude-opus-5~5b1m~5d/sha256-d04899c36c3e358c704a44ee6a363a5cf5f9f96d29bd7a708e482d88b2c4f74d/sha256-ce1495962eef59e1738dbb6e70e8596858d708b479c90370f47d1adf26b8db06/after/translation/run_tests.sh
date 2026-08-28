#!/usr/bin/env bash
# Build the C .so, build the Rust cdylib for BOTH profiles (the tests dlopen
# `target/<profile>/libdriver.so`, and `cargo test` alone does not refresh it),
# check symbol parity, then run the whole differential suite.
#
# Usage: ./run_tests.sh [extra cargo feature flags...]
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cd "$here" || exit 2

feature_args=("$@")
label="${feature_args[*]:-<default features>}"
echo "############ configuration: $label"

# --- C reference -----------------------------------------------------------
if [ ! -f "$root/c_src/build/libdriver.so" ]; then
    echo "== building the C reference"
    ( mkdir -p "$root/c_src/build" && cd "$root/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || exit 2
fi

# --- Rust cdylib, both profiles -------------------------------------------
for profile in "" "--release"; do
    # shellcheck disable=SC2086
    if ! cargo build --offline $profile "${feature_args[@]}" > "$here/target/build.$$.log" 2>&1; then
        echo "BUILD FAILED ($profile $label)"; tail -30 "$here/target/build.$$.log"; rm -f "$here/target/build.$$.log"; exit 1
    fi
done
rm -f "$here/target/build.$$.log"

# --- symbol parity ---------------------------------------------------------
"$here/check_symbols.sh" "$here/target/release/libdriver.so" || exit 1
"$here/check_symbols.sh" "$here/target/debug/libdriver.so"   || exit 1

# --- differential tests ----------------------------------------------------
# The whole suite is run twice: once against target/debug/libdriver.so and once
# against target/release/libdriver.so (the shipped artifact, optimised and built
# with panic = "abort").
rc=0
for so in debug release; do
    echo "== differential suite vs target/$so/libdriver.so"
    out="$(DIFF_RUST_SO="$here/target/$so/libdriver.so" \
        cargo test --offline "${feature_args[@]}" -- --test-threads=8 2>&1)"
    status=$?
    printf '%s\n' "$out" | grep -E '^test result:|^error|panicked at|FAILED' | head -20
    if [ "$status" -ne 0 ]; then
        printf '%s\n' "$out" | tail -40
        rc=1
    fi
done
if [ "$rc" -eq 0 ]; then echo "DIFFERENTIAL SUITE: OK ($label)"; fi
exit "$rc"
