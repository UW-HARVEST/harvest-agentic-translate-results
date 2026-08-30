#!/usr/bin/env bash
# Build the C sources (c_src/, never modified) as shared libraries, one per
# (OP, REPEAT) configuration.  Mirrors c_src/CMakeLists.txt, which compiles with
#   CMAKE_C_FLAGS = -DOP=${OP} -DREPEAT=${REPEAT}
# but produces a .so from mdcore.c instead of the `driver` executable so the
# exported symbols can be dlopen'ed by the differential tests.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRC="$ROOT/c_src/src"
OUT="${1:-$ROOT/cbuild}"
mkdir -p "$OUT"

OPS=(add sub mul)
REPEATS=(0 1 2 3 4 5 6 7)

for op in "${OPS[@]}"; do
  for rep in "${REPEATS[@]}"; do
    gcc -O2 -fPIC -shared -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/libcdriver_${op}_${rep}.so" "$SRC/mdcore.c"
    # Also build the full driver executable for end-to-end stdout comparison.
    gcc -O2 -DOP="$op" -DREPEAT="$rep" \
        -o "$OUT/cdriver_${op}_${rep}" "$SRC/mdcore.c" "$SRC/mdmain.c"
  done
done

echo "built $(ls "$OUT" | wc -l) artifacts in $OUT"
