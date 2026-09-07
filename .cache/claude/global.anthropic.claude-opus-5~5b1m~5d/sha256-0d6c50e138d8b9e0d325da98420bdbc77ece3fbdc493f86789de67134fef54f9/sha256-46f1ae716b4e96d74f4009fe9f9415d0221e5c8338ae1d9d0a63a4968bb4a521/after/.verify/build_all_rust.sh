#!/bin/bash
# Compile the Rust cdylib, driver and all test binaries for every feature combo
# and stage them per-combo so the actual test RUN can be parallelised (cargo
# itself cannot run concurrently against one target dir).
set -u
ROOT=$HARVEST_WORKDIR
CRATE=$ROOT/translation
OUT=$ROOT/.verify/bins
mkdir -p "$OUT"
cd "$CRATE"
fail=0
for b in haraka sha2 shake blake; do for t in robust simple; do for s in 128s 128f 192s 192f 256s 256f; do
  combo="$b,$t,$s"; d="$OUT/${b}_${t}_${s}"
  rm -rf "$d"; mkdir -p "$d"
  cargo build --offline --release --no-default-features --features "$combo" >/dev/null 2>&1 \
    || { echo "BUILD FAIL $combo"; fail=$((fail+1)); continue; }
  cp target/release/libsphincs_plus.so "$d/" || { echo "NO CDYLIB $combo"; fail=$((fail+1)); }
  D="$d" cargo test --offline --release --no-default-features --features "$combo" --no-run \
      --message-format=json 2>/dev/null \
    | D="$d" python3 -c '
import json,sys,shutil,os
out=os.environ["D"]
n=0
for line in sys.stdin:
    try: m=json.loads(line)
    except Exception: continue
    if m.get("reason")=="compiler-artifact" and m.get("profile",{}).get("test") and m.get("executable"):
        exe=m["executable"]
        shutil.copy2(exe, os.path.join(out, os.path.basename(exe).rsplit("-",1)[0]))
        n+=1
print(n)
' > "$d/.ntests"
  cp target/release/driver "$d/driver" || { echo "NO DRIVER $combo"; fail=$((fail+1)); }
  n=$(cat "$d/.ntests")
  [ "$n" -ge 8 ] || { echo "ONLY $n TEST BINS $combo"; fail=$((fail+1)); }
done; done; done
echo "stage failures: $fail"
ls "$OUT" | wc -l
