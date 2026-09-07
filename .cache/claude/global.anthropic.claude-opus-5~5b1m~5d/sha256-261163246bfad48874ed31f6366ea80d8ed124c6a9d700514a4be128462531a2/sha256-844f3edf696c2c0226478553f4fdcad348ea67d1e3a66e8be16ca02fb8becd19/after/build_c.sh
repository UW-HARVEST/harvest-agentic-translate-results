#!/bin/bash
# Builds the C sources (unmodified, from c_src/) as shared libraries and
# executables for every (OP, REPEAT) configuration, into ./cbuild.
# Nothing inside c_src/ is written to.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
SRC="$ROOT/c_src/src"
OUT="$ROOT/cbuild"
mkdir -p "$OUT"

OPS="${OPS:-add sub mul}"
REPS="${REPS:-0 1 2 3 4 5 6 7}"

for op in $OPS; do
  for rep in $REPS; do
    tag="${op}_${rep}"
    # shared library: mdcore.c only (mdmain.c holds main())
    gcc -DOP=$op -DREPEAT=$rep -fPIC -shared -O2 \
        -I"$SRC" -o "$OUT/libcdriver_$tag.so" "$SRC/mdcore.c"
    # executable, same flags CMake would use
    gcc -DOP=$op -DREPEAT=$rep -fPIE -pie -O2 \
        -I"$SRC" -o "$OUT/cdriver_$tag" "$SRC/mdcore.c" "$SRC/mdmain.c"
  done
done
echo "built: $(ls "$OUT" | wc -l) artifacts in $OUT"
