#!/usr/bin/env bash
# Negative control for the differential suite.
#
# Injects one deliberate divergence at a time into src/lib.rs and checks the
# Phase B / Phase C tests react as predicted. A suite that never fails is
# indistinguishable from a suite that never checks anything, so this is what
# makes "all tests pass" meaningful.
#
# Each mutant carries an EXPECTATION:
#   catch  - the mutant changes observable behaviour; the suite MUST fail.
#   equiv  - the mutant is provably UNOBSERVABLE through the public ABI (see
#            the proof next to each one); the suite MUST still pass. If such a
#            mutant is "caught", the proof is wrong and must be revisited.
#
# src/lib.rs is always restored from .verify/lib.rs.pristine, including on
# interrupt.
set -uo pipefail
cd "$(dirname "$0")"

PRISTINE=.verify/lib.rs.pristine
[[ -f $PRISTINE ]] || { echo "missing $PRISTINE"; exit 1; }
: "${TMPDIR:=/tmp}"

# Refuse to run if src/lib.rs has real changes not yet folded into the pristine
# snapshot — otherwise this script's restore step would silently discard them.
if ! cmp -s src/lib.rs "$PRISTINE"; then
  echo "src/lib.rs differs from $PRISTINE."
  echo "If the difference is intentional, refresh the snapshot first:"
  echo "    cp src/lib.rs $PRISTINE"
  diff -u "$PRISTINE" src/lib.rs | head -40
  exit 1
fi

restore() { cp "$PRISTINE" src/lib.rs; }
trap restore EXIT INT TERM
restore

run_tests() {
  timeout 600 cargo test --offline -q --test phase_b_valid --test phase_c_errors \
    >"$TMPDIR/mut.log" 2>&1
}

mutate() { # <search> <replace>
  python3 - "$1" "$2" <<'PY'
import sys
s = open('src/lib.rs').read()
old, new = sys.argv[1], sys.argv[2]
if old not in s:
    sys.exit(2)
open('src/lib.rs', 'w').write(s.replace(old, new, 1))
PY
}

declare -a NAMES=() EXPECTS=() OLDS=() NEWS=()
add() { NAMES+=("$1"); EXPECTS+=("$2"); OLDS+=("$3"); NEWS+=("$4"); }

# --- observable payload / arithmetic decisions -------------------------------

add "M1  lm_dot2 y-term dst=left" catch \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))' \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_lhs(a.y, b.y))'

add "M2  lm_dot2 addss dst=left" catch \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))' \
    'add_dst_lhs_MUT(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))'

add "M3  lm_dot2 naive a.x*b.x + a.y*b.y" catch \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))' \
    'a.x * b.x + a.y * b.y'

add "M14 lm_dot2 x-term dst=right" catch \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))' \
    'add_dst_rhs(mul_dst_rhs(a.x, b.x), mul_dst_rhs(a.y, b.y))'

add "M13 lm_sub2 dst=right (subtrahend wins)" catch \
    'lm_v2(sub_dst_lhs(a.x, b.x), sub_dst_lhs(a.y, b.y))' \
    'lm_v2(sub_dst_rhs_MUT(a.x, b.x), sub_dst_rhs_MUT(a.y, b.y))'

add "M5  return (v,u) instead of (u,v)" catch \
    '    lm_v2(u, v)
}' \
    '    lm_v2(v, u)
}'

add "M6  swap v0/v1 (p2 <-> p3)" catch \
    '    let v0: lm_vec2 = lm_sub2(p3, p1);
    let v1: lm_vec2 = lm_sub2(p2, p1);' \
    '    let v0: lm_vec2 = lm_sub2(p2, p1);
    let v1: lm_vec2 = lm_sub2(p3, p1);'

add "M7  wrong quiet bit (0x0020_0000)" catch \
    'f32::from_bits(x.to_bits() | 0x0040_0000)' \
    'f32::from_bits(x.to_bits() | 0x0020_0000)'

add "M17 quiet() also clears the sign bit" catch \
    'f32::from_bits(x.to_bits() | 0x0040_0000)' \
    'f32::from_bits((x.to_bits() | 0x0040_0000) & 0x7FFF_FFFF)'

add "M9  u-numerator muls commuted" catch \
    'sub_dst_lhs(mul_dst_lhs(dot11, dot02), mul_dst_lhs(dot01, dot12))' \
    'sub_dst_lhs(mul_dst_lhs(dot02, dot11), mul_dst_lhs(dot12, dot01))'

add "M15 u-numerator subss dst=right" catch \
    'sub_dst_lhs(mul_dst_lhs(dot11, dot02), mul_dst_lhs(dot01, dot12))' \
    'sub_dst_rhs_MUT(mul_dst_lhs(dot11, dot02), mul_dst_lhs(dot01, dot12))'

add "M16 v-numerator muls commuted" catch \
    'sub_dst_lhs(mul_dst_lhs(dot00, dot12), mul_dst_lhs(dot01, dot02))' \
    'sub_dst_lhs(mul_dst_lhs(dot12, dot00), mul_dst_lhs(dot02, dot01))'

add "M12 final mul by invDenom dst=right" catch \
    '    let u: f32 = mul_dst_lhs(
        sub_dst_lhs(mul_dst_lhs(dot11, dot02), mul_dst_lhs(dot01, dot12)),
        invDenom,
    );' \
    '    let u: f32 = mul_dst_rhs(
        sub_dst_lhs(mul_dst_lhs(dot11, dot02), mul_dst_lhs(dot01, dot12)),
        invDenom,
    );'

add "M11 f64 intermediates in lm_dot2" catch \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))' \
    '((a.x as f64 * b.x as f64) + (a.y as f64 * b.y as f64)) as f32'

add "M18 fused multiply-add in lm_dot2" catch \
    'add_dst_rhs(mul_dst_lhs(a.x, b.x), mul_dst_rhs(a.y, b.y))' \
    'a.y.mul_add(b.y, a.x * b.x)'

# --- provably UNOBSERVABLE variants (expected to survive) --------------------
#
# M4: `sub_dst_lhs(l, r)` and `l - r` are the same function on x86-64: SUBSS /
#     VSUBSS put the minuend in the destination / first source operand, so the
#     hardware already implements "if lhs is NaN -> Q(lhs) else if rhs is NaN ->
#     Q(rhs)". The helper only documents that; it cannot change the result.
add "M4  lm_sub2 naive subtraction" equiv \
    'lm_v2(sub_dst_lhs(a.x, b.x), sub_dst_lhs(a.y, b.y))' \
    'lm_v2(a.x - b.x, a.y - b.y)'

# M8: the denominator's `dot00*dot11` payload can never reach the output.
#     mul_dst_lhs and mul_dst_rhs differ only when BOTH operands are NaN, i.e.
#     when dot00 and dot11 are both NaN. But then the u-numerator starts with
#     mul_dst_lhs(dot11, dot02) = Q(dot11) and the v-numerator with
#     mul_dst_lhs(dot00, dot12) = Q(dot00); a NaN lhs wins the following subss
#     and the final mulss, so u = Q(dot11) and v = Q(dot00) regardless of what
#     the denominator (and hence invDenom) held.
add "M8  denominator mul dst=right" equiv \
    'sub_dst_lhs(mul_dst_lhs(dot00, dot11), mul_dst_lhs(dot01, dot01))' \
    'sub_dst_lhs(mul_dst_rhs(dot00, dot11), mul_dst_lhs(dot01, dot01))'

# M10: div_dst_lhs and div_dst_rhs differ only when both operands are NaN. The
#      dividend here is the literal 1.0f, which is never NaN.
add "M10 divss dst=right (1.0f / denom)" equiv \
    'div_dst_lhs(
        1.0f32,' \
    'div_dst_rhs_MUT(
        1.0f32,'

# M19: the denominator's subss destination is likewise unobservable. It matters
#      only when both `dot00*dot11` and `dot01*dot01` are NaN. A square is NaN
#      only if its operand is, so `dot01*dot01` is NaN only when dot01 is NaN —
#      and a NaN dot01 makes BOTH numerators NaN (each contains a
#      mul_dst_lhs(dot01, .) whose NaN lhs wins), which masks invDenom.
add "M19 denominator subss dst=right" equiv \
    'sub_dst_lhs(mul_dst_lhs(dot00, dot11), mul_dst_lhs(dot01, dot01))' \
    'sub_dst_rhs_MUT(mul_dst_lhs(dot00, dot11), mul_dst_lhs(dot01, dot01))'

# M20: dot01*dot01 has identical operands, so lhs/rhs choice cannot matter.
add "M20 dot01*dot01 dst=right" equiv \
    'mul_dst_lhs(dot01, dot01)' \
    'mul_dst_rhs(dot01, dot01)'

HELPERS='
#[inline]
fn add_dst_lhs_MUT(lhs: f32, rhs: f32) -> f32 {
    if lhs.is_nan() { quiet(lhs) } else if rhs.is_nan() { quiet(rhs) } else { lhs + rhs }
}
#[inline]
fn div_dst_rhs_MUT(lhs: f32, rhs: f32) -> f32 {
    if rhs.is_nan() { quiet(rhs) } else if lhs.is_nan() { quiet(lhs) } else { lhs / rhs }
}
#[inline]
fn sub_dst_rhs_MUT(lhs: f32, rhs: f32) -> f32 {
    if rhs.is_nan() { quiet(rhs) } else if lhs.is_nan() { quiet(lhs) } else { lhs - rhs }
}
'

bad=0
printf '%-42s %-7s %-9s %s\n' "MUTANT" "EXPECT" "OBSERVED" "VERDICT"
printf '%.0s-' {1..78}; echo
for i in "${!NAMES[@]}"; do
  restore
  if ! mutate "${OLDS[$i]}" "${NEWS[$i]}"; then
    printf '%-42s %-7s %-9s %s\n' "${NAMES[$i]}" "${EXPECTS[$i]}" "SKIP" "FAIL (pattern not found)"
    bad=$((bad + 1))
    continue
  fi
  printf '%s' "$HELPERS" >> src/lib.rs   # unused helpers are #[inline], harmless
  if run_tests; then observed=survived; else observed=caught; fi

  if [[ ${EXPECTS[$i]} == catch && $observed == caught ]]; then
    verdict="OK"
  elif [[ ${EXPECTS[$i]} == equiv && $observed == survived ]]; then
    verdict="OK (unobservable, as proved)"
  else
    verdict="FAIL"
    bad=$((bad + 1))
  fi
  printf '%-42s %-7s %-9s %s\n' "${NAMES[$i]}" "${EXPECTS[$i]}" "$observed" "$verdict"
done
restore

printf '%.0s-' {1..78}; echo
if [[ $bad -eq 0 ]]; then
  echo "mutation check PASSED: ${#NAMES[@]}/${#NAMES[@]} mutants behaved as predicted"
else
  echo "mutation check FAILED: $bad mutant(s) did not behave as predicted"
fi
exit $bad
