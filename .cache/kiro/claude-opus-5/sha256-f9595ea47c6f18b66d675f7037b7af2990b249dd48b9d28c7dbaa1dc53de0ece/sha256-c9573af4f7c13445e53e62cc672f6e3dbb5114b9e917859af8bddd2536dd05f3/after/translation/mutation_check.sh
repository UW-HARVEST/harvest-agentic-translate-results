#!/usr/bin/env bash
# Negative control for the differential suite (validates Phases B and C).
#
# Injects a series of subtle, individually-plausible mistranslations into
# src/lib.rs, runs the suite after each, and requires that every one is caught.
# A differential suite that cannot detect a deliberate bug proves nothing, so
# this is the meta-test that validates the tests.
#
# Mutations are applied with `perl -0777 -pe` (whole-file, so multi-line edits
# work). Entries marked EQUIV: are known *equivalent* mutants — they change the
# text but provably not the behaviour, so "not caught" is the correct outcome
# and is what the script asserts.
#
# src/lib.rs is restored from a backup after every mutation and on exit.
set -uo pipefail
cd "$(dirname "$0")"

BAK=$(mktemp)
cp src/lib.rs "$BAK"
restore() { cp "$BAK" src/lib.rs; rm -f "$BAK"; }
trap restore EXIT

# name | perl expression
MUTATIONS=(
  "strict-< to <= on d2 selection|s/if d2 < d0 \{/if d2 <= d0 {/"
  "strict-< to <= on d1 selection|s/if d1 < d0 \{/if d1 <= d0 {/"
  "swap selection order (d2 tested first)|s/if d1 < d0 \{\n        uni = uni1;\n    \}\n    if d2 < d0 \{\n        uni = uni2;\n    \}/if d2 < d0 {\n        uni = uni2;\n    }\n    if d1 < d0 {\n        uni = uni1;\n    }/"
  "d3 >> 5 becomes >> 4|s/d3 >> 5/d3 >> 4/g"
  "d0 sign-shift 31 becomes 30|s/d0 \^= d0 >> 31/d0 ^= d0 >> 30/"
  "abs-minus-1 idiom replaced by real abs|s/d0 \^= d0 >> 31;/d0 = d0.wrapping_abs();/"
  "value mask 7 becomes 15|s/& 7\)/\& 15)/g"
  "sign-bit mask 8 becomes 16|s/& 8\) != 0/\& 16) != 0/g"
  "clamp mask ~7 becomes ~15|s/& !7i32/\& !15i32/g"
  "lsbit == 4 becomes lsbit == 2|s/if lsbit == 4 \{/if lsbit == 2 {/"
  "lsbit&1 parity test inverted|s/\(lsbit & 1\) != 0/(lsbit \& 1) == 0/"
  "divide by 8 becomes divide by 4|s|/ 8;|/ 4;|g"
  "2*k+1 becomes 2*k+0|s/\.wrapping_add\(1\)\n        \.wrapping_mul\(step\)/.wrapping_add(0)\n        .wrapping_mul(step)/g"
  "uni2 = uni - 2 instead of uni - 1|s/uni\.wrapping_sub\(1\)/uni.wrapping_sub(2)/"
  "uni1 diff negation dropped|s/if \(uni1 & 8\) != 0 \{/if false {/"
  "wrapping_mul(step) -> saturating_mul|s/\.wrapping_mul\(step\)/.saturating_mul(step)/g"
  "d0 measured against tgt2 instead of tgt|s/let mut d0: i32 = tgt\.wrapping_sub\(p0\);/let mut d0: i32 = tgt2.wrapping_sub(p0);/"
  "clamp guard inverted for uni1|s/if \(\(uni \^ uni1\) & !7i32\) != 0/if ((uni ^ uni1) \& !7i32) == 0/"
  "lsbit==4 candidate uni2 not adjusted|s/uni2 \|= \(uni2 >> 1\) & \(uni2 >> 2\) & 1;//"
  "EQUIV:arith->logical shift under a & 1 mask|s/uni \|= \(uni >> 1\) & \(uni >> 2\) & 1;/uni |= ((((uni as u32) >> 1) as i32) \& (((uni as u32) >> 2) as i32)) \& 1;/"
)

real_caught=0; real_missed=0; equiv_ok=0; equiv_bad=0; unusable=0
printf '%-52s %s\n' "MUTATION" "RESULT"
printf '%.0s-' {1..78}; echo

for m in "${MUTATIONS[@]}"; do
  name="${m%%|*}"; expr="${m#*|}"
  expect_equiv=0
  if [[ "$name" == EQUIV:* ]]; then expect_equiv=1; name="${name#EQUIV:}"; fi

  cp "$BAK" src/lib.rs
  perl -0777 -pe "$expr" "$BAK" > src/lib.rs 2>/dev/null || {
    printf '%-52s %s\n' "$name" "PERL-ERROR"; unusable=$((unusable+1)); continue; }
  if cmp -s "$BAK" src/lib.rs; then
    printf '%-52s %s\n' "$name" "NO-OP (pattern matched nothing)"; unusable=$((unusable+1)); continue
  fi

  out=$(timeout 600 cargo test 2>&1)
  if echo "$out" | grep -qE 'error\[E|error: could not compile'; then
    printf '%-52s %s\n' "$name" "DID-NOT-COMPILE"; unusable=$((unusable+1)); continue
  fi
  nfail=$(echo "$out" | grep -cE '^test .* FAILED')

  if [ "$expect_equiv" -eq 1 ]; then
    if [ "$nfail" -eq 0 ]; then
      printf '%-52s %s\n' "$name" "equivalent mutant, not caught (expected)"; equiv_ok=$((equiv_ok+1))
    else
      printf '%-52s %s\n' "$name" "*** caught, but expected EQUIVALENT ***"; equiv_bad=$((equiv_bad+1))
    fi
  else
    if [ "$nfail" -gt 0 ]; then
      printf '%-52s %s\n' "$name" "CAUGHT ($nfail tests failed)"; real_caught=$((real_caught+1))
    else
      printf '%-52s %s\n' "$name" "*** NOT CAUGHT ***"; real_missed=$((real_missed+1))
    fi
  fi
done

cp "$BAK" src/lib.rs
printf '%.0s-' {1..78}; echo
echo "real bugs caught      : $real_caught"
echo "real bugs MISSED      : $real_missed"
echo "equivalent mutants ok : $equiv_ok"
echo "equivalent surprises  : $equiv_bad"
echo "unusable probes       : $unusable"
echo
echo "--- suite re-run on the restored (unmutated) source ---"
timeout 600 cargo test 2>&1 | tail -3

if [ "$real_missed" -ne 0 ] || [ "$unusable" -ne 0 ]; then
  echo "FAIL: mutation battery has missed bugs or unusable probes"; exit 1
fi
echo "OK: every non-equivalent mutation was detected"
