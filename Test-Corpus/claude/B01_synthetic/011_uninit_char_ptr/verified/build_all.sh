#!/usr/bin/env bash
# Builds every artifact the differential test-suite needs:
#
#   c_src/build/driver              the C executable (via c_src/CMakeLists.txt)
#   target/csrc/libcdriver.so       c_src/src/main.c built as a shared library
#   target/<profile>/driver         the Rust executable
#   target/<profile>/libdriver.so   the Rust cdylib
#   target/<profile>/examples/so_runner
#
# Nothing inside c_src/ is modified: the .so lands in target/.
#
# Usage: ./build_all.sh [--release] [--features <list>] [--no-default-features]
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"

CARGO_ARGS=()
PROFILE_DIR=debug
for a in "$@"; do
  case "$a" in
    --release) PROFILE_DIR=release ;;
  esac
  CARGO_ARGS+=("$a")
done

echo "==> C executable (cmake, default = unoptimised)"
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
test -x c_src/build/driver

echo "==> C shared library (same TU, same default flags)"
mkdir -p target/csrc
gcc -shared -fPIC -o target/csrc/libcdriver.so c_src/src/main.c
test -f target/csrc/libcdriver.so

echo "==> Rust bin + cdylib + so_runner  (cargo ${CARGO_ARGS[*]:-<default>})"
cargo build --offline "${CARGO_ARGS[@]}" >/dev/null
cargo build --offline --examples "${CARGO_ARGS[@]}" >/dev/null
test -x "target/$PROFILE_DIR/driver"
test -f "target/$PROFILE_DIR/libdriver.so"
test -x "target/$PROFILE_DIR/examples/so_runner"

echo "==> Symbol parity (nm -D)"
nm -D --defined-only target/csrc/libcdriver.so | awk '{print $3}' | sort >target/csrc/c.syms
nm -D --defined-only "target/$PROFILE_DIR/libdriver.so" | awk '{print $3}' | sort >target/csrc/rust.syms
MISSING=$(comm -23 target/csrc/c.syms target/csrc/rust.syms || true)
echo "    C exports  : $(tr '\n' ' ' <target/csrc/c.syms)"
if [ -n "$MISSING" ]; then
  echo "    MISSING from Rust .so: $MISSING" >&2
  exit 1
fi
echo "    missing from Rust .so: none (0)"

echo "==> ldd sanity"
if ldd "target/$PROFILE_DIR/libdriver.so" | grep -q "not found"; then
  echo "    unresolved shared-library dependency" >&2
  exit 1
fi
echo "    all dynamic dependencies resolve"
echo "OK  ($ROOT, profile=$PROFILE_DIR)"
