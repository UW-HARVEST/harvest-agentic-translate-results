#!/bin/bash
# cargo check every valid feature combination (mirrors the CMake cache variable
# cross-product: HASH_BACKEND x THASH x SECPAR, plus the randombytes provider).
set -u
cd "$(dirname "$0")"
mkdir -p /tmp/checklogs
fail=0
n=0
for b in blake haraka sha2 shake shake256; do
  for t in robust simple; do
    for s in 128s 128f 192s 192f 256s 256f; do
      for r in "" ",urandom"; do
        combo="$b,$t,$s$r"
        tag=$(echo "$combo" | tr ',' '_')
        n=$((n+1))
        if ! cargo check --offline --no-default-features --features "$combo" \
              > /tmp/checklogs/$tag.log 2>&1; then
          echo "CHECK FAIL: $combo"
          grep -m5 '^error' /tmp/checklogs/$tag.log
          fail=1
        fi
      done
    done
  done
done
echo "checked $n combinations, fail=$fail"
exit $fail
