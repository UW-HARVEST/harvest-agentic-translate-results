#!/bin/bash
# cargo check every valid feature combination (HASH_BACKEND x THASH x SECPAR x rng)
cd "$(dirname "$0")"
fail=0
mkdir -p /tmp/chklogs
: > /tmp/check_results.txt
for b in blake haraka sha2 shake; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      for r in "" ",urandom"; do
        combo="$b,$t,$s$r"
        tag="${b}_${t}_${s}${r//,/_}"
        if timeout 300 cargo check --offline --no-default-features --features "$combo" \
             > /tmp/chklogs/$tag.log 2>&1; then
          echo "OK   $combo" >> /tmp/check_results.txt
        else
          echo "FAIL $combo" >> /tmp/check_results.txt
          fail=1
        fi
      done
    done
  done
done
echo "OK:   $(grep -c '^OK' /tmp/check_results.txt)"
echo "FAIL: $(grep -c '^FAIL' /tmp/check_results.txt)"
grep '^FAIL' /tmp/check_results.txt
exit $fail
