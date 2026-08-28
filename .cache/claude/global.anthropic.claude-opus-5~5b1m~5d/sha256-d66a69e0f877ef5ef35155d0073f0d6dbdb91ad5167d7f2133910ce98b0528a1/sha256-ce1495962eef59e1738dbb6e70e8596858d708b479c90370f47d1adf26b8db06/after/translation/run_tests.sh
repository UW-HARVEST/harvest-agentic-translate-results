#!/usr/bin/env bash
# Differential test driver.
#
# The Rust `.so` under test is `target/<profile>/libagglom_lib.so`. Because the
# crate is `crate-type = ["cdylib"]` only, the integration tests do NOT depend
# on it in cargo's graph — so it must be built explicitly BEFORE `cargo test`,
# otherwise a stale `.so` is exercised.
set -euo pipefail
cd "$(dirname "$0")"

C_BUILD=../c_src/build
if [ ! -d "$C_BUILD" ] || ! ls "$C_BUILD"/*.so >/dev/null 2>&1; then
    echo "==> building the C shared library"
    (cd ../c_src && mkdir -p build && cd build \
        && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
        && cmake --build . >/dev/null)
fi

# Enumerate every feature combination declared in Cargo.toml (there are none,
# so this yields just the default and the no-default-features build).
FEATURES=$(cargo read-manifest 2>/dev/null \
    | tr ',' '\n' | grep -o '"features":{[^}]*}' || true)
echo "==> declared features: ${FEATURES:-<none>}"

run_combo() {
    local profile="$1"; shift
    local label="$1"; shift
    echo
    echo "############################################################"
    echo "# profile=$profile  features=$label"
    echo "############################################################"
    local flags=()
    [ "$profile" = "release" ] && flags+=(--release)
    cargo build "${flags[@]}" "$@"
    cargo test "${flags[@]}" "$@" -- --test-threads="${TEST_THREADS:-4}"
}

run_combo release "default"
run_combo release "no-default-features"  --no-default-features
run_combo dev     "default"
run_combo dev     "no-default-features"  --no-default-features

echo
echo "==> ALL COMBINATIONS PASSED"
