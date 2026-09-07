#!/bin/bash
# Mutation sanity check: prove the differential suite actually detects
# behavioural divergence from the C. Each mutant must be CAUGHT.
set -u
cd "$(dirname "$0")/.."
BAK=.scratch/lib.rs.bak

run_mut () {
  name="$1"; old="$2"; new="$3"
  cp "$BAK" src/lib.rs
  OLD="$old" NEW="$new" python3 -c '
import os
p="src/lib.rs"
s=open(p).read()
old,new=os.environ["OLD"],os.environ["NEW"]
assert old in s, "pattern not found: "+old
open(p,"w").write(s.replace(old,new,1))
' || { echo "MUTANT $name: PATTERN NOT FOUND"; return; }
  # MUST build the cdylib before testing, or the .so is stale and every
  # mutant "survives" vacuously.
  out=$(timeout 600 ./run_tests.sh 2>&1)
  # A mutant counts as CAUGHT if any test fails OR the harness aborts (a
  # mutated out-of-bounds store shows up as SIGSEGV, not as a clean assert).
  if echo "$out" | grep -qE '^test result: FAILED|SIGSEGV|SIGABRT|error: test failed'; then
    nf=$(echo "$out" | grep -cE '^test .* FAILED' || true)
    sig=$(echo "$out" | grep -oE 'SIGSEGV|SIGABRT' | head -1 || true)
    echo "MUTANT [$name]: CAUGHT (${nf} failing test(s)${sig:+, $sig})"
    echo "$out" | grep -E '^test .* FAILED' | head -4 | sed 's/^/       /'
  else
    echo "MUTANT [$name]: *** SURVIVED ***"
  fi
}

run_mut "drop s -= (s<0)"      's -= (s < 0) as i16;'           's -= 0;'
run_mut "clamp-hi >= becomes >" 'if sample as f64 >= 32766.5 {' 'if sample as f64 > 32766.5 {'
run_mut "clamp-lo <= becomes <" 'if sample as f64 <= -32767.5 {' 'if sample as f64 < -32767.5 {'
run_mut "clamp-hi const 32766.5->32767.5" '>= 32766.5 {' '>= 32767.5 {'
run_mut "coeff 213 -> 214"     '* 213f32;'                      '* 214f32;'
run_mut "coeff 75038 -> 75037" '* 75038f32;'                    '* 75037f32;'
run_mut "acc2 coeff -5 -> 5"   '* -5f32;'                       '* 5f32;'
run_mut "nch offset unsigned"  '16isize * nch as isize'         '16isize * nch as u32 as isize'
run_mut "store index 16 -> 32" '*pcm.offset(16isize * nch as isize)' '*pcm.offset(32isize * nch as isize)'
run_mut "z += 2 becomes z += 1" 'z.offset(2)'                    'z.offset(1)'
run_mut "reassociate acc order" 'a += (unsafe { g(z, 12 * 64) } - unsafe { g(z, 2 * 64) }) * 459f32;' \
                                'a = a + (unsafe { g(z, 12 * 64) } * 459f32) - (unsafe { g(z, 2 * 64) } * 459f32);'
run_mut "saturating float cast" '(sample + 0.5f32) as i32 as i16' '(sample + 0.5f32) as i16'
run_mut "clamp to i16::MAX-1"  'return 32767i16;'               'return 32766i16;'
run_mut "acc1/acc2 store swap" 'let z = unsafe { z.offset(2) };' 'let z = unsafe { z.offset(2) }; let _ = &z;'
run_mut "f64 accumulator"      'let mut a: f32;' 'let mut a: f64;'
run_mut "z index 64 -> 63"     'g(z, 14 * 64)' 'g(z, 14 * 63)'
run_mut "acc2 tap 6 sign flip" '* -9975f32;' '* 9975f32;'
run_mut "round .5 -> .4"       '(sample + 0.5f32)' '(sample + 0.4f32)'
run_mut "clamp-lo -32768->-32767" 'return -32768i16;' 'return -32767i16;'

cp "$BAK" src/lib.rs
echo "--- restored, verifying clean tree passes ---"
timeout 600 ./run_tests.sh 2>&1 | grep -E '^test result'
