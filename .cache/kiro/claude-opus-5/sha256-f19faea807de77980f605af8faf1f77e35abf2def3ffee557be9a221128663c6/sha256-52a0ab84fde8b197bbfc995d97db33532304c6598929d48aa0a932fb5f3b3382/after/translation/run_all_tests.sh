#!/bin/bash
# Phase B + C + D across every feature combination.
#
# `cargo build` runs first with the same features so that
# target/release/libsphincsplus.so is the object the tests dlopen; the harness
# also asserts the loaded object matches the test binary's configuration, so a
# stale artifact fails loudly rather than silently passing.
set -u
cd "$(dirname "$0")"
mkdir -p /tmp/testlogs
RESULTS=test_results.txt
: > "$RESULTS"

BACKENDS=${BACKENDS:-"blake haraka sha2 shake"}
THASHES=${THASHES:-"robust simple"}
SECPARS=${SECPARS:-"128s 128f 192s 192f 256s 256f"}
RNGS=${RNGS:-", ,urandom"}   # comma separated list of suffixes
TESTARGS=${TESTARGS:-}

fail=0
for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      for r in "" ",urandom"; do
        combo="$b,$t,$s$r"
        tag="${b}_${t}_${s}${r//,/_}"
        log=/tmp/testlogs/$tag.log
        start=$(date +%s)
        if ! timeout 300 cargo build --release --offline --no-default-features \
              --features "$combo" > "$log" 2>&1; then
          echo "BUILDFAIL $combo" | tee -a "$RESULTS"
          fail=1
          continue
        fi
        if timeout 580 cargo test --release --offline --no-default-features \
              --features "$combo" $TESTARGS -- --test-threads=1 >> "$log" 2>&1; then
          n=$(grep -c '^test .* ok$' "$log")
          echo "PASS $combo tests=$n secs=$(( $(date +%s) - start ))" | tee -a "$RESULTS"
        else
          echo "FAIL $combo secs=$(( $(date +%s) - start ))" | tee -a "$RESULTS"
          grep -E "^test .* FAILED|panicked at|^error" "$log" | head -10
          fail=1
        fi
      done
    done
  done
done

echo "------------------------------------------------------------"
echo "PASS: $(grep -c '^PASS' "$RESULTS")   FAIL: $(grep -cE '^(FAIL|BUILDFAIL)' "$RESULTS")"
grep -E '^(FAIL|BUILDFAIL)' "$RESULTS"
exit $fail
