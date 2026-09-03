#!/usr/bin/env bash
# Build the C reference as (a) a shared library exposing mdcore.c's surface and
# (b) the cmake `driver` executable, for every OP x REPEAT configuration.
#
# c_src/ is never modified: sources are read in place and every artifact lands
# in ./cbuild/.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$here/cbuild"
rm -rf "$out"
mkdir -p "$out"

for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    tag="${OP}_${R}"
    # Shared library: mdcore.c only (mdmain.c holds `main`), mirroring the
    # symbol surface of the Rust cdylib.
    gcc -O2 -fPIC -shared \
        -DOP="$OP" -DREPEAT="$R" \
        -I"$here/c_src/src" \
        -o "$out/libcdriver_${tag}.so" \
        "$here/c_src/src/mdcore.c"
    # Reference executable, same flags cmake uses.
    mkdir -p "$out/exe_${tag}"
    gcc -O2 -DOP="$OP" -DREPEAT="$R" \
        -I"$here/c_src/src" \
        -o "$out/exe_${tag}/driver" \
        "$here/c_src/src/mdcore.c" "$here/c_src/src/mdmain.c"
  done
done

echo "built $(ls "$out"/libcdriver_*.so | wc -l) shared libs and $(ls -d "$out"/exe_* | wc -l) executables in $out"
