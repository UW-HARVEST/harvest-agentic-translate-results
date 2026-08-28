#!/usr/bin/env bash
# Harness sensitivity check ("are the tests actually able to fail?").
#
# Injects deliberate bugs into translation/src/lib.rs one at a time, rebuilds
# both .so artifacts and runs the suite, asserting that the suite FAILS for
# every mutation. Restores the pristine source afterwards.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$HERE/src/lib.rs"
BAK="$(mktemp)"
cp "$SRC" "$BAK"
restore() { cp "$BAK" "$SRC"; rm -f "$BAK"; }
trap restore EXIT

FAIL=0

# run_mutation <name> <perl-expr>            -> suite MUST fail  (real bug)
# run_equivalent_mutation <name> <perl-expr>  -> suite MUST pass  (control:
#     a source change that provably cannot alter observable behaviour)
_apply_and_test() {
  local name="$1" expr="$2"
  cp "$BAK" "$SRC"
  perl -0777 -i -pe "$expr" "$SRC"
  if cmp -s "$BAK" "$SRC"; then
    echo "[$name] MUTATION DID NOT APPLY (pattern did not match) -- inconclusive"
    return 2
  fi
  ( cd "$HERE" && cargo build --offline >/dev/null 2>&1 \
                && cargo build --offline --release >/dev/null 2>&1 ) || {
    echo "[$name] mutant did not compile -- inconclusive"; return 2; }
  ( cd "$HERE" && cargo test --offline >/dev/null 2>&1 )
}

run_mutation() {
  local name="$1"; shift
  _apply_and_test "$name" "$1"
  case $? in
    0) echo "[$name] *** SUITE STILL PASSED -- TESTS ARE BLIND TO THIS BUG ***"; FAIL=1 ;;
    2) FAIL=1 ;;
    *) echo "[$name] suite correctly FAILED  ✅" ;;
  esac
}

run_equivalent_mutation() {
  local name="$1"; shift
  _apply_and_test "$name" "$1"
  case $? in
    0) echo "[$name] suite correctly PASSED (behaviourally equivalent)  ✅" ;;
    2) FAIL=1 ;;
    *) echo "[$name] *** SUITE FAILED ON AN EQUIVALENT MUTANT (false positive) ***"; FAIL=1 ;;
  esac
}

echo "=== mutation sensitivity check ==="

# M1: "fix" the deliberate Pfn10 >>3 quirk to match case 10 (>>4)
run_mutation "M1 Pfn10 >>3 -> >>4" \
  's/(NOTE: the C source shifts by 3 here.*?)5i32\.wrapping_mul\(p0\)\.wrapping_sub\(p1\) >> 3/$1 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 4/s'

# M2: "fix" the deliberate Pfn11 >>1 quirk to match case 11 (>>3)
run_mutation "M2 Pfn11 >>1 -> >>3" \
  's/(NOTE: the C source shifts by 1 here.*?)p0\.wrapping_add\(p1\) >> 1/$1 p0.wrapping_add(p1) >> 3/s'

# M3: replace the truncating /16 in Pfn7 with an arithmetic shift
run_mutation "M3 Pfn7 /16 -> >>4" \
  's/\.wrapping_add\(1i32\.wrapping_mul\(ps\(psamp, idx, 5\)\)\)\n            \.wrapping_div\(16\)/.wrapping_add(1i32.wrapping_mul(ps(psamp, idx, 5))) >> 4/s'

# M4: wrong FIR row index (ignore pfcn-12, always use row 0)
run_mutation "M4 firfx row index -> 0" \
  's/\(\*ridx\)\.firfx\[\(pfcn - 12\) as usize\]/(*ridx).firfx[0usize]/s'

# M5: off-by-one in the dispatch table (pfcn 11 falls through to generic)
run_mutation "M5 drop GetPredictFunc case 11" \
  's/        11 => BTAC1C2_PredictSample_Pfn11,\n//s'

# M6a: call_predict's `default:` returns 1 -> out-of-range selectors accepted
run_mutation "M6a call_predict default -> 1" \
  's/        _ => \{\}\n    \}\n    result/        _ => { result = 1; }\n    }\n    result/s'

# M6b: mis-wired comparison for pfcn 0 (compares against Pfn1)
run_mutation "M6b call_predict case0 compares Pfn1" \
  's/        0 => result = \(fcn == BTAC1C2_PredictSample_Pfn0 as \*mut c_void\) as c_int,/        0 => result = (fcn == BTAC1C2_PredictSample_Pfn1 as *mut c_void) as c_int,/s'

# M6c (control): adding `12` to the `11` arm cannot change anything, because for
# pfcn 12 GetPredictFunc yields the *generic* predictor, which still compares
# unequal to Pfn11 -> result stays 0. The suite must therefore still pass.
run_equivalent_mutation "M6c call_predict 11|12 arm (equivalent)" \
  's/        11 => result = \(fcn == BTAC1C2_PredictSample_Pfn11 as \*mut c_void\) as c_int,/        11 | 12 => result = (fcn == BTAC1C2_PredictSample_Pfn11 as *mut c_void) as c_int,/s'

# M7: generic switch case 10 uses >>3 (i.e. quirk removed the other way)
run_mutation "M7 generic case10 >>4 -> >>3" \
  's/pred = 5i32\.wrapping_mul\(p0\)\.wrapping_sub\(p1\) >> 4;/pred = 5i32.wrapping_mul(p0).wrapping_sub(p1) >> 3;/s'

# M8: drop the & 7 masking (real OOB / wrong-slot read)
run_mutation "M8 remove & 7 masking" \
  's/let idx = i\.wrapping_sub\(k\) & 7;/let idx = i.wrapping_sub(k) \& 15;/s'

# M9: generic default returns 1 instead of 0
run_mutation "M9 generic default -> 1" \
  's/            _ => \{\n                pred = 0;\n            \}/            _ => {\n                pred = 1;\n            }/s'

# M10: sign error in Pfn9 coefficient
run_mutation "M10 Pfn9 coefficient sign" \
  's/\.wrapping_sub\(4i32\.wrapping_mul\(ps\(psamp, idx, 6\)\)\)\n            \.wrapping_add\(4i32\.wrapping_mul\(ps\(psamp, idx, 7\)\)\)/.wrapping_add(4i32.wrapping_mul(ps(psamp, idx, 6))).wrapping_sub(4i32.wrapping_mul(ps(psamp, idx, 7)))/s'

restore
trap - EXIT
( cd "$HERE" && cargo build --offline >/dev/null 2>&1 && cargo build --offline --release >/dev/null 2>&1 )

echo
if [ "$FAIL" -eq 0 ]; then
  echo "harness is sensitive to every injected bug  ✅"
else
  echo "harness has blind spots (see above)"
fi
exit "$FAIL"
