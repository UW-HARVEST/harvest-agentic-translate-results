#!/bin/bash
# Build the C shared library + executable and the Rust shared library + executable
# for one (OP, REPEAT) configuration.  Nothing under c_src/ is modified.
#
# usage: ./build_all.sh <op> <repeat>
set -e
OP=${1:-add}
REPEAT=${2:-5}
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/artifacts/${OP}_${REPEAT}"
mkdir -p "$OUT"

# ---- C shared library (mdcore.c only; mdmain.c holds main()) ----
# Flags match CMAKE_C_FLAGS from c_src/CMakeLists.txt exactly (no -O level),
# plus the -fPIC/-shared needed to package the same objects as a library.
gcc -shared -fPIC -DOP="$OP" -DREPEAT="$REPEAT" \
    -I"$ROOT/c_src/src" \
    -o "$OUT/libcdriver.so" "$ROOT/c_src/src/mdcore.c"

# ---- C executable, via the project's own CMake build ----
CBUILD="$ROOT/artifacts/cbuild_${OP}_${REPEAT}"
mkdir -p "$CBUILD"
(cd "$CBUILD" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    -DOP="$OP" -DREPEAT="$REPEAT" >/dev/null && cmake --build . >/dev/null)
cp "$CBUILD/driver" "$OUT/cdriver"

# ---- Rust shared library + executable ----
(cd "$ROOT" && cargo build --quiet --no-default-features --features "$OP,$REPEAT")
cp "$ROOT/target/debug/libdriver.so" "$OUT/librdriver.so"
cp "$ROOT/target/debug/driver" "$OUT/rdriver"

echo "$OUT"
