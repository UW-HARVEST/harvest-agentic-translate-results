#!/usr/bin/env bash
# Build BOTH libraries, then run the differential suite.
#
# `cargo test` does not rebuild the cdylib (the integration tests dlopen it
# rather than linking it), so the release .so MUST be built explicitly first or
# the suite silently tests a stale artifact. tests/common/mod.rs also guards
# against this, but building here means it never comes up.
set -euo pipefail
cd "$(dirname "$0")"

echo "== building C reference =="
(cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null)

echo "== building Rust cdylib (release) =="
cargo build --release

echo "== running differential suite =="
cargo test --release "$@"
