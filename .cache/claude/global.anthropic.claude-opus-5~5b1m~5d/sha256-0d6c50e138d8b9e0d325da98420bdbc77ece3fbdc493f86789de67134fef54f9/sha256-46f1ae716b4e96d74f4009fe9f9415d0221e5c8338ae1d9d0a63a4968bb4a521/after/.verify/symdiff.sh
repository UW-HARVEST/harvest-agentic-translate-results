set -u
ROOT=$HARVEST_WORKDIR
cd $ROOT/translation
for b in haraka sha2 shake blake; do for t in robust simple; do for s in 128s 128f 192s 192f 256s 256f; do
  cargo build --release --no-default-features --features "$b,$t,$s" > /dev/null 2>&1 || { echo "RUSTBUILD FAIL $b $t $s"; continue; }
  d=$ROOT/.verify/cbuilds/${b}_${t}_${s}
  cat <(nm -D --defined-only $d/app/libsphincs_core.so) <(nm -D --defined-only $d/app/libsphincs_core_det.so) <(nm -D --defined-only $d/lib/$b/lib$b.so) | awk 'NF==3{print $3}' | sort -u > $ROOT/.verify/c_${b}_${t}_${s}.txt
  nm -D --defined-only target/release/libsphincs_plus.so | awk 'NF==3{print $3}' | sort -u > $ROOT/.verify/r_${b}_${t}_${s}.txt
  miss=$(comm -23 $ROOT/.verify/c_${b}_${t}_${s}.txt $ROOT/.verify/r_${b}_${t}_${s}.txt | tr '\n' ' ')
  if [ -n "$miss" ]; then echo "MISSING [$b,$t,$s]: $miss"; else echo "OK  [$b,$t,$s]"; fi
done; done; done
