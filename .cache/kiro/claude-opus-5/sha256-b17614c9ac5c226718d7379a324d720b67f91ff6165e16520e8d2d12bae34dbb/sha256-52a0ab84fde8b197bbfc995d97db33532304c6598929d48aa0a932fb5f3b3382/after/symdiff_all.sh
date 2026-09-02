#!/bin/bash
# For every (backend, thash, secpar) build the Rust cdylib and diff its exported
# dynamic symbols against the union of the C reference shared libraries.
set -u
W="$(cd "$(dirname "$0")" && pwd)"
OUT="$W/rsbuild"
mkdir -p "$OUT"
fail=0
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
     for r in "" ",urandom"; do
      if [ -z "$r" ]; then tag="${b}_${t}_${s}"; core="libsphincs_core_det.so"; else tag="${b}_${t}_${s}_urandom"; core="libsphincs_core.so"; fi
      (cd "$W/translation" && timeout 300 cargo build --release --offline \
          --no-default-features --features "$b,$t,$s$r" > "$OUT/build_${tag}.log" 2>&1) \
        || { echo "RS BUILD FAIL $tag"; tail -20 "$OUT/build_${tag}.log"; fail=1; continue; }
      cp "$W/translation/target/release/libsphincsplus.so" "$OUT/libsphincsplus_${tag}.so"

      cdir="$W/cbuild/${b}_${t}_${s}"
      cat <(nm -D --defined-only "$cdir/$core") \
          <(nm -D --defined-only "$cdir/lib${b}.so") \
        | awk '$2=="T"||$2=="B"||$2=="D"||$2=="R"||$2=="W"{print $3}' | LC_ALL=C sort -u > "$OUT/c_${tag}.syms"
      nm -D --defined-only "$OUT/libsphincsplus_${tag}.so" \
        | awk '$2=="T"||$2=="B"||$2=="D"||$2=="R"||$2=="W"{print $3}' | LC_ALL=C sort -u > "$OUT/rs_${tag}.syms"
      miss=$(comm -23 "$OUT/c_${tag}.syms" "$OUT/rs_${tag}.syms")
      if [ -n "$miss" ]; then
        echo "MISSING [$tag]: $(echo $miss | tr '\n' ' ')"
        fail=1
      fi
     done
    done
  done
done
echo "symbol parity check done over $(ls $OUT/rs_*.syms | wc -l) configurations, fail=$fail"
exit $fail
