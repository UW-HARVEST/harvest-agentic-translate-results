#!/bin/bash
# Run Phase B + Phase C differential tests for every (HASH_BACKEND, THASH,
# SECPAR) combination, plus the end-to-end KAT driver comparison (CONFIGS.md
# row 56).
#
# Prerequisite: ./build_matrix.sh  (builds the C .so's, the Rust .so's, the
# drivers and the ground-truth params.txt for all 48 combos into /tmp/dif/).
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT/translation"

BACKENDS="${BACKENDS:-blake haraka sha2 shake}"
THASHES="${THASHES:-simple robust}"
SECPARS="${SECPARS:-128f 128s 192f 192s 256f 256s}"
TESTS="${TESTS:---test configs --test backends --test errors}"

pass=0; fail=0; failed=""
# Expected number of #[test]s actually executed per binary, so that a silently
# skipped or filtered test binary cannot masquerade as a pass.
EXP_CONFIGS=33   # 34 minus the #[ignore]d zz_rs_only_fingerprint helper
EXP_BACKENDS=21
EXP_ERRORS=25

for b in $BACKENDS; do
  for t in $THASHES; do
    for s in $SECPARS; do
      combo="${b}_${t}_${s}"
      log=/tmp/dif/$combo/test.log

      if ! SPX_DIF_DIR=/tmp/dif/$combo timeout 600 \
           cargo test --release --offline --no-default-features \
                --features "$b,$t,$s" $TESTS > "$log" 2>&1; then
        echo "TEST FAIL $combo"
        grep -E "^test .* FAILED|panicked at|C != Rust|left:|right:" "$log" | head -12
        fail=$((fail+1)); failed="$failed $combo"
        continue
      fi

      # every binary must have reported the expected number of passes
      bad_count=0
      for pair in "configs.rs:$EXP_CONFIGS" "backends.rs:$EXP_BACKENDS" "errors.rs:$EXP_ERRORS"; do
        f=${pair%%:*}; want=${pair##*:}
        got=$(awk -v f="$f" '
          $0 ~ ("Running tests/" f) {on=1; next}
          on && /test result:/ {print $4; on=0}' "$log")
        if [ "$got" != "$want" ]; then
          echo "COUNT FAIL $combo: tests/$f reported '${got:-none}' passed, expected $want"
          bad_count=1
        fi
      done
      if [ $bad_count -ne 0 ]; then
        fail=$((fail+1)); failed="$failed $combo"; continue
      fi

      # CONFIGS.md row 56: the two KAT drivers must print the same digest
      cd=$(LD_LIBRARY_PATH=/tmp/osslib timeout 600 /tmp/dif/$combo/c_driver 2>&1)
      rd=$(timeout 600 /tmp/dif/$combo/rs_driver 2>&1)
      if [ "$cd" != "$rd" ]; then
        echo "KAT FAIL $combo"; echo "  C : $cd"; echo "  RS: $rd"
        fail=$((fail+1)); failed="$failed $combo"; continue
      fi

      echo "ok $combo  ($((EXP_CONFIGS+EXP_BACKENDS+EXP_ERRORS)) tests, $cd)"
      pass=$((pass+1))
    done
  done
done
echo "-----------------------------------------------"
echo "combos passed: $pass   failed: $fail$failed"
exit $fail
