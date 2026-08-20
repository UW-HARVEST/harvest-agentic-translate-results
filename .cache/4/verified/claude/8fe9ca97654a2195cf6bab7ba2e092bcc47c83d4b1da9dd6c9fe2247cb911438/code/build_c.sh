#!/bin/sh
# Builds everything the differential test suite needs:
#
#   cbuild/libcdriver.so  -- the C library:  exactly the translation units that
#                            c_src/CMakeLists.txt compiles (src/q_math.c +
#                            src/main.c, -Iinc -Isrc, -lm), linked as a shared
#                            object.  This is the symbol-parity reference.
#   cbuild/libcwrap.so    -- src/q_math.c + tests/csupport/wrappers.c, which adds
#                            `w_*` entry points for the header's macros and
#                            `static ID_INLINE` functions.
#   cbuild/cdriver        -- the C executable, via CMake, exactly as documented.
#
# c_src/ is never modified.
set -e
cd "$(dirname "$0")"
mkdir -p cbuild

CFLAGS="-Ic_src/inc -Ic_src/src"

# Same flags CMake uses for the (unset) default build type: none.
gcc -shared -fPIC $CFLAGS -o cbuild/libcdriver.so \
    c_src/src/q_math.c c_src/src/main.c -lm

gcc -shared -fPIC $CFLAGS -o cbuild/libcwrap.so \
    c_src/src/q_math.c tests/csupport/wrappers.c -lm

# The documented CMake build of the C driver executable.
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
cp c_src/build/driver cbuild/cdriver

echo "built:"
ls -l cbuild/
