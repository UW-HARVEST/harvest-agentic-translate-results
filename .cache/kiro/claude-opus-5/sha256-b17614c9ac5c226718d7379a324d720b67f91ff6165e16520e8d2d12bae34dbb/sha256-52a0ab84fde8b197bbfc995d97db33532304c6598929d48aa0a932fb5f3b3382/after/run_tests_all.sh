#!/bin/bash
# Run the differential test suites (Phase B + Phase C) for every feature
# combination: 4 backends x 2 thash x 6 secpar x {DRBG, urandom} = 96.
#
# `randombytes()`/`DRBG_ctx` are process-global state in rng.c, so the tests must
# run single-threaded.
#
# Usage: ./run_tests_all.sh [backend-filter] [thash-filter] [secpar-filter]
set -u
W="$(cd "$(dirname "$0")" && pwd)"
cd "$W/translation"
mkdir -p "$W/testlogs"

BF="${1:-blake haraka sha2 shake}"
TF="${2:-robust simple}"
SF="${3:-128s 128f 192s 192f 256s 256f}"

fail=0
pass=0
for b in $BF; do
 for t in $TF; do
  for s in $SF; do
   for r in "" "urandom"; do
    if [ -z "$r" ]; then feats="$b,$t,$s"; tag="${b}_${t}_${s}"; else feats="$b,$t,$s,$r"; tag="${b}_${t}_${s}_$r"; fi
    log="$W/testlogs/$tag.log"
    # The cdylib must be rebuilt explicitly: `cargo test` does not always
    # regenerate a crate-type=["cdylib"] artifact, and the harness dlopens it.
    if ! timeout 600 cargo build --release --offline --no-default-features \
           --features "$feats" > "$log" 2>&1; then
      echo "BUILD FAIL  $tag"; tail -n 15 "$log"; fail=$((fail+1)); continue
    fi
    if RUST_TEST_THREADS=1 timeout 600 cargo test --release --offline \
         --no-default-features --features "$feats" >> "$log" 2>&1; then
      n=$(grep -c '^test .* ok$' "$log")
      echo "PASS  $tag  ($n tests)"
      pass=$((pass+1))
    else
      echo "TEST FAIL   $tag"
      grep -E 'panicked|differ|FAILED|test result' "$log" | head -n 20
      fail=$((fail+1))
    fi
   done
  done
 done
done
echo "-----------------------------------------------"
echo "combinations passed: $pass, failed: $fail"
exit $((fail > 0))
