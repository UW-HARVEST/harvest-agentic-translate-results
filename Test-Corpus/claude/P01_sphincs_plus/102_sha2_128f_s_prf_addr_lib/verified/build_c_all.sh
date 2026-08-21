#!/bin/bash
# Build the C reference shared libraries for every (backend, thash, secpar) combo.
set -u
R="$(cd "$(dirname "$0")" && pwd)"
OUT="$R/cbuild"
mkdir -p "$OUT"
LOG="${TMPDIR:-/var/tmp}/build_c_all.log"
: > "$LOG"
fail=0
for bk in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      d="$OUT/$bk-$th-$sp"
      if [ -f "$d/app/libsphincs_core_det.so" ]; then continue; fi
      mkdir -p "$d"
      ( cd "$d" && cmake "$R/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DHASH_BACKEND=$bk -DTHASH=$th -DSECPAR=$sp >>"$LOG" 2>&1 \
        && cmake --build . -j 4 >>"$LOG" 2>&1 ) || { echo "FAIL $bk $th $sp"; fail=1; }
    done
  done
done
echo "build_c_all done fail=$fail"
