#!/bin/bash
# Run every staged test binary + the driver stdout comparison for ONE combo.
# Usage: run_combo.sh <backend> <thash> <secpar>
set -u
ROOT=$HARVEST_WORKDIR
b=$1; t=$2; s=$3; combo="$b,$t,$s"
d=$ROOT/.verify/bins/${b}_${t}_${s}
clib=$ROOT/.verify/clibs/libspx_c_${b}_${t}_${s}.so
cdir=$ROOT/.verify/cbuilds/${b}_${t}_${s}
log=$ROOT/.verify/logs/${b}_${t}_${s}.log
mkdir -p "$ROOT/.verify/logs"
: > "$log"
ok=1

export SPX_C_LIB="$clib"
export SPX_RUST_LIB="$d/libsphincs_plus.so"

for bin in layer0_utils layer0_hash layer1_keyed layer2_trees layer3_scheme layer4_api rng errors; do
  if [ ! -x "$d/$bin" ]; then echo "MISSING BIN $bin" >> "$log"; ok=0; continue; fi
  if ! timeout 900 "$d/$bin" --test-threads=4 >> "$log" 2>&1; then
    echo "FAILED BIN $bin" >> "$log"; ok=0
  fi
done

# Phase B row 60: driver stdout byte-for-byte + exit code.
cout=$(cd "$cdir" && LD_LIBRARY_PATH="app:lib/$b:$ROOT/.verify/osslib" timeout 900 ./app/driver 2>>"$log"); crc=$?
rout=$(timeout 900 "$d/driver" 2>>"$log"); rrc=$?
if [ "$cout" != "$rout" ] || [ "$crc" != "$rrc" ]; then
  { echo "DRIVER MISMATCH"; echo "  C   (rc=$crc): $cout"; echo "  Rust(rc=$rrc): $rout"; } >> "$log"
  ok=0
else
  echo "DRIVER OK rc=$crc $cout" >> "$log"
fi

# Phase D: symbol parity for this combo.
cat <(nm -D --defined-only "$cdir/app/libsphincs_core.so") \
    <(nm -D --defined-only "$cdir/app/libsphincs_core_det.so") \
    <(nm -D --defined-only "$cdir/lib/$b/lib$b.so") \
  | awk 'NF==3{print $3}' | sort -u > "$ROOT/.verify/logs/csym_${b}_${t}_${s}.txt"
nm -D --defined-only "$d/libsphincs_plus.so" | awk 'NF==3{print $3}' | sort -u \
  > "$ROOT/.verify/logs/rsym_${b}_${t}_${s}.txt"
miss=$(comm -23 "$ROOT/.verify/logs/csym_${b}_${t}_${s}.txt" "$ROOT/.verify/logs/rsym_${b}_${t}_${s}.txt" | tr '\n' ' ')
if [ -n "$miss" ]; then echo "SYMBOLS MISSING: $miss" >> "$log"; ok=0; else echo "SYMBOLS OK" >> "$log"; fi

npass=$(grep -oE '[0-9]+ passed' "$log" | awk '{x+=$1} END{print x+0}')
if [ $ok -eq 1 ]; then echo "PASS $combo ($npass tests)"; else echo "FAIL $combo -- see $log"; fi
[ $ok -eq 1 ]
