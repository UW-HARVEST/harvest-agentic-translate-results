#!/bin/bash
# Verify all 48 (backend, thash, secpar) combinations of the Rust translation
# against the expected KAT digests captured from the C reference.
set -u
export CARGO_NET_OFFLINE=true
pass=0
fail=0
for bk in haraka sha2 shake blake; do
  for th in robust simple; do
    for sp in 128s 128f 192s 192f 256s 256f; do
      cargo build --release --no-default-features --features "$bk $th $sp" >"${TMPDIR:-/var/tmp}/va.log" 2>&1 || {
        echo "BUILD FAIL $bk $th $sp"; cat "${TMPDIR:-/var/tmp}/va.log"; fail=$((fail+1)); continue; }
      got=$(./target/release/driver | grep -oE '[0-9A-F]{64}')
      exp=$(grep "^$bk $th $sp " expected_kat.txt | grep -oE '[0-9A-F]{64}')
      if [ "$got" = "$exp" ]; then
        pass=$((pass+1))
      else
        fail=$((fail+1))
        echo "MISMATCH $bk $th $sp: got=$got exp=$exp"
      fi
    done
  done
done
echo "TOTAL pass=$pass fail=$fail"
