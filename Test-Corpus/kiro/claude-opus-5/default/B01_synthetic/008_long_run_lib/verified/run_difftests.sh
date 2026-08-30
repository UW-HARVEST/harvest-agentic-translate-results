#!/usr/bin/env bash
# Build the C reference library and the Rust cdylib, then run the fast
# differential test suite.
#
# `cargo test` alone is not enough: this package's only lib target is a
# `cdylib`, and cargo does not emit that artifact when it builds integration
# tests, so the .so has to be produced explicitly first.
#
# Usage: ./run_difftests.sh [--debug] [extra cargo test args...]
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

profile_args=(--release)
profile_dir=release
if [[ "${1:-}" == "--debug" ]]; then
    profile_args=()
    profile_dir=debug
    shift
fi

echo "== building C reference library =="
mkdir -p "$root/c_src/build"
(
    cd "$root/c_src/build"
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
    cmake --build .
)

echo "== building Rust cdylib ($profile_dir) =="
cd "$here"
cargo build "${profile_args[@]}"
test -f "target/$profile_dir/liblong.so"

echo "== running differential tests ($profile_dir) =="
cargo test "${profile_args[@]}" "$@"
