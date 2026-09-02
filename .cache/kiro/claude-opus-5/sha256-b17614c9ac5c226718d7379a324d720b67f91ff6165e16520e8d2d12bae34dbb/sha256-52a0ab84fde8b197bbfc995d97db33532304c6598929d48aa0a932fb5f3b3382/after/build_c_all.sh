#!/bin/bash
# Build the C reference shared libraries for every (HASH_BACKEND, THASH, SECPAR)
# combination into cbuild/<backend>_<thash>_<secpar>/.
set -u
W="$(cd "$(dirname "$0")" && pwd)"
OUT="$W/cbuild"
mkdir -p "$OUT"
fail=0
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      tag="${b}_${t}_${s}"
      d="$OUT/$tag"
      if [ -f "$d/lib${b}.so" ] && [ -f "$d/libsphincs_core_det.so" ] && [ -f "$d/libsphincs_core.so" ]; then
        continue
      fi
      rm -rf "$d"; mkdir -p "$d"
      (cd "$d" && cmake "$W/c_src" \
          -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DHASH_BACKEND=$b -DTHASH=$t -DSECPAR=$s \
          -DCMAKE_C_FLAGS="-I$W/c_compat" \
          -DCMAKE_EXE_LINKER_FLAGS="-L$W/c_compat/lib" \
          -DCMAKE_SHARED_LINKER_FLAGS="-L$W/c_compat/lib" > cmake.log 2>&1 \
        && cmake --build . -j4 > build.log 2>&1) || { echo "C BUILD FAIL $tag"; tail -5 "$d/build.log"; fail=1; continue; }
      cp "$d/lib/$b/lib${b}.so" "$d/" 2>/dev/null
      cp "$d/app/libsphincs_core.so" "$d/app/libsphincs_core_det.so" "$d/" 2>/dev/null
      cp "$d/app/driver" "$d/" 2>/dev/null
    done
  done
done
echo "C libs built: $(ls -d $OUT/*/ | wc -l) dirs, fail=$fail"
exit $fail
