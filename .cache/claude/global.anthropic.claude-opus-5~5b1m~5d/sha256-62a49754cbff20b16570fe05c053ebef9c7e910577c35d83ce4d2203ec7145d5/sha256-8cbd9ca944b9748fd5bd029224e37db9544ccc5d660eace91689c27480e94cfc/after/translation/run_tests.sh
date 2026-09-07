#!/usr/bin/env bash
# Differential test runner.
#
# MUST be used instead of a bare `cargo test`: this crate's only lib target is a
# `cdylib`, and `cargo test` does NOT rebuild a cdylib-only lib target. Running
# `cargo test` directly would load a stale `.so` and every differential
# assertion would silently pass against outdated code. (tests/common/mod.rs also
# hard-fails on a stale `.so` as a backstop.)
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
profile="${PROFILE:-release}"

# ---- 1. build the C reference shared library -------------------------------
mkdir -p "$root/c_src/build"
(
  cd "$root/c_src/build"
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null
  cmake --build . >/dev/null
)
echo "C   .so: $(ls "$root"/c_src/build/*.so)"

# ---- 2. build the Rust cdylib (the thing cargo test would skip) ------------
# `CARGO_FEATURE_ARGS` (e.g. "--no-default-features --features foo") applies to
# BOTH the build and the test run so they stay in sync; "$@" goes to the test
# run only.
cd "$here"
read -r -a feat <<<"${CARGO_FEATURE_ARGS:-}"
relflag=()
[ "$profile" = release ] && relflag=(--release)

cargo build "${relflag[@]}" "${feat[@]}" >/dev/null
echo "Rust .so: $here/target/$profile/libomni_collide_lib.so"

# ---- 3. run the differential tests ----------------------------------------
cargo test "${relflag[@]}" "${feat[@]}" --no-fail-fast "$@"
