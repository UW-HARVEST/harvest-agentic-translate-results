#!/bin/bash
# End-to-end check of the `driver` KAT transcript: run the C reference driver
# (built by ./build_c_all.sh) and the Rust driver for every (backend, thash,
# secpar) combination and compare the 32-byte transcript digests.
#
# This covers app/src/PQCgenKAT_sign.c -> src/main.rs, which the .so-level
# differential tests do not reach.
set -u
R="$(cd "$(dirname "$0")" && pwd)"
export CARGO_NET_OFFLINE=true
LOG="${TMPDIR:-/var/tmp}/kat_all.log"; : > "$LOG"
pass=0; fail=0
for bk in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      d="$R/cbuild/$bk-$th-$sp"
      cdrv="$d/app/driver"
      [ -x "$cdrv" ] || { echo "MISSING C driver $bk $th $sp"; fail=$((fail+1)); continue; }
      cgot=$(LD_LIBRARY_PATH="$d/app:$d/lib/$bk" "$cdrv" 2>>"$LOG" | grep -oE '[0-9A-F]{64}')
      cd "$R" || exit 1
      cargo build --release --no-default-features --features "$bk $th $sp" >>"$LOG" 2>&1 \
        || { echo "RUST BUILD FAIL $bk $th $sp"; fail=$((fail+1)); continue; }
      rgot=$(./target/release/driver 2>>"$LOG" | grep -oE '[0-9A-F]{64}')
      exp=$(grep "^$bk $th $sp " "$R/expected_kat.txt" | grep -oE '[0-9A-F]{64}')
      if [ -n "$cgot" ] && [ "$cgot" = "$rgot" ] && [ "$cgot" = "$exp" ]; then
        pass=$((pass+1)); printf 'OK   %-22s %s\n' "$bk-$th-$sp" "$cgot"
      else
        fail=$((fail+1))
        printf 'FAIL %-22s C=%s R=%s exp=%s\n' "$bk-$th-$sp" "$cgot" "$rgot" "$exp"
      fi
    done
  done
done
echo "kat_all: pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
