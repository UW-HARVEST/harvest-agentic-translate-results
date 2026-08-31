#!/bin/bash
# Builds the C reference for every (OP, REPEAT) configuration:
#   cbuild/libcdriver_<op>_<rep>.so   (shared library from mdcore.c)
#   cbuild/driver_<op>_<rep>          (executable from mdcore.c + mdmain.c)
#
# c_src/ is never modified; all output goes to ./cbuild.
set -u
root="$(cd "$(dirname "$0")" && pwd)"
src="$root/c_src/src"
out="$root/cbuild"
mkdir -p "$out"

# The CMake build (as documented in the task) for the default configuration.
if [ "${WITH_CMAKE:-1}" = 1 ]; then
  mkdir -p "$root/c_src/build"
  (cd "$root/c_src/build" \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON -DOP=add -DREPEAT=5 >/dev/null \
    && cmake --build . >/dev/null) || { echo "cmake build failed"; exit 1; }
fi

fail=0
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    gcc -O2 -fPIC -shared -DOP="$op" -DREPEAT="$rep" \
        -o "$out/libcdriver_${op}_${rep}.so" "$src/mdcore.c" || fail=1
    gcc -O2 -fPIE -pie -DOP="$op" -DREPEAT="$rep" \
        -o "$out/driver_${op}_${rep}" "$src/mdcore.c" "$src/mdmain.c" || fail=1
  done
done

# `OP`/`REPEAT` unset must behave exactly like the #ifndef defaults (add / 5);
# build those too so the "no feature" Rust combination is compared against a C
# object that likewise had no -D flags.
gcc -O2 -fPIC -shared -o "$out/libcdriver_default.so" "$src/mdcore.c" || fail=1
gcc -O2 -fPIE -pie -o "$out/driver_default" "$src/mdcore.c" "$src/mdmain.c" || fail=1

echo "built $(ls "$out" | wc -l) C artifacts in $out"
exit $fail
