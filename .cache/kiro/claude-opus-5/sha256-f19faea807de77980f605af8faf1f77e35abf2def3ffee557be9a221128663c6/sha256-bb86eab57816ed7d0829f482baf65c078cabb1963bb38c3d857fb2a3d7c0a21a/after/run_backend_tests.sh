#!/bin/bash
# Phase B + C over the whole build-time cross-product, for ONE backend.
#
#   ./run_backend_tests.sh <backend>
#
# THASH x SECPAR x {default, urandom} = 2 * 6 * 2 = 24 configurations.
# A per-backend CARGO_TARGET_DIR lets the four backends run in parallel without
# fighting over the cargo build lock.  Tests share the process-global DRBG
# state, so --test-threads=1 is required.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
b="$1"
export CARGO_TARGET_DIR="$ROOT/translation/target-$b"
LOGDIR=/tmp/difftests
mkdir -p $LOGDIR
: > $LOGDIR/summary_$b.txt
cd "$ROOT/translation"
fail=0
for t in robust simple; do
  for s in 128f 192f 256f 128s 192s 256s; do
    for extra in "" ",urandom"; do
      feats="$b,$t,$s$extra"
      tag=$(echo "$feats" | tr ',' '_')
      export SPHINCS_RUST_SO="$CARGO_TARGET_DIR/release/libsphincsplus.so"
      if ! timeout 600 cargo build --release --offline --no-default-features \
            --features "$feats" > $LOGDIR/build_$tag.log 2>&1; then
        echo "BUILDFAIL $tag" | tee -a $LOGDIR/summary_$b.txt; fail=1; continue
      fi
      if timeout 900 cargo test --release --offline --no-default-features \
            --features "$feats" -- --test-threads=1 \
            > $LOGDIR/test_$tag.log 2>&1; then
        n=$(grep -c '^test .* ok$' $LOGDIR/test_$tag.log)
        echo "PASS $tag ($n tests)" >> $LOGDIR/summary_$b.txt
      else
        echo "FAIL $tag" >> $LOGDIR/summary_$b.txt
        fail=1
      fi
    done
  done
done
echo "$b done: $(grep -c '^PASS' $LOGDIR/summary_$b.txt) pass, $(grep -cE '^(FAIL|BUILDFAIL)' $LOGDIR/summary_$b.txt) fail"
exit $fail
