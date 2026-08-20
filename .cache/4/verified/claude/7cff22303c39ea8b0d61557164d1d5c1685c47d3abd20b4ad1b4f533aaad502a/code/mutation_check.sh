#!/usr/bin/env bash
# Sensitivity check for the differential suite.
#
# Each entry below is a plausible mis-translation of `c_src/src/lib.c`.
# Every mutant is declared either:
#   CATCH       - the suite MUST fail on it (a real behavioural difference), or
#   EQUIVALENT  - the mutant is provably indistinguishable through the public
#                 API, so the suite MUST still pass. The reason is given, and
#                 the reason itself is asserted against the C `.so` by
#                 tests/nan_masking.rs.
#
# The script fails if any verdict differs from the declaration.
#
# Usage: ./mutation_check.sh
set -u
cd "$(dirname "$0")"

LIB=src/lib.rs
BAK="$(mktemp "${TMPDIR:-/tmp}/lib.rs.bak.XXXXXX")"
cp "$LIB" "$BAK"
restore() { cp "$BAK" "$LIB"; }
cleanup() { restore; rm -f "$BAK"; }
trap cleanup EXIT

fail=0
n_catch=0
n_equiv=0

mutate() {
    local expect="$1" name="$2" from="$3" to="$4"
    cp "$BAK" "$LIB"
    if ! python3 - "$LIB" "$from" "$to" <<'PY'
import sys
p, a, b = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p).read()
if a not in s:
    sys.exit("pattern not found")
open(p, 'w').write(s.replace(a, b, 1))
PY
    then
        echo "ERROR       $name (mutation pattern not found in $LIB)"; fail=1; return
    fi
    if ! cargo build --offline --release >/dev/null 2>&1; then
        echo "ERROR       $name (mutant does not compile)"; fail=1; return
    fi
    if timeout 600 cargo test --offline --release >/dev/null 2>&1; then
        got=EQUIVALENT
    else
        got=CATCH
    fi
    if [ "$got" = "$expect" ]; then
        if [ "$expect" = CATCH ]; then
            echo "ok  caught        $name"; n_catch=$((n_catch+1))
        else
            echo "ok  equivalent    $name"; n_equiv=$((n_equiv+1))
        fi
    else
        if [ "$expect" = CATCH ]; then
            echo "FAIL survived     $name   <-- the suite is insensitive here"
        else
            echo "FAIL caught       $name   <-- declared equivalent but observable"
        fi
        fail=1
    fi
}

# ---------------------------------------------------------------------------
# Mutants that MUST be caught.
# ---------------------------------------------------------------------------

mutate CATCH "final '+ 4*dxy*dxy' add not commuted (accumulator as destination)" \
    'fadd(fmul(fmul(FOUR, dxy), dxy), sqd)' \
    'fadd(sqd, fmul(fmul(FOUR, dxy), dxy))'

mutate CATCH "clamp uses f32::max (normalises NaN and -0.0)" \
    'let clamped = if flt(sqd, ZERO) { ZERO } else { sqd };' \
    'let clamped = f32::from_bits(sqd).max(0.0).to_bits();'

mutate CATCH "clamp comparison made non-strict" \
    'if flt(sqd, ZERO) { ZERO } else { sqd }' \
    'if !flt(ZERO, sqd) { ZERO } else { sqd }'

mutate CATCH "manufactured NaN uses 0x7fc00000 instead of the x86 indefinite" \
    'const INDEFINITE: u32 = 0xffc0_0000;' \
    'const INDEFINITE: u32 = 0x7fc0_0000;'

mutate CATCH "NaN propagation prefers the source operand over the destination" \
    'if is_nan_bits(a) {
        Some(quiet(a))
    } else if is_nan_bits(b) {
        Some(quiet(b))' \
    'if is_nan_bits(b) {
        Some(quiet(b))
    } else if is_nan_bits(a) {
        Some(quiet(a))'

mutate CATCH "NaN operands are not quieted" \
    'fn quiet(x: u32) -> u32 {
    x | QUIET_BIT' \
    'fn quiet(x: u32) -> u32 {
    x'

mutate CATCH "branch guard is <= instead of <" \
    'f32::from_bits(a) < f32::from_bits(b)
}' \
    'f32::from_bits(a) <= f32::from_bits(b)
}'

mutate CATCH "if/else arms swap their dest slots" \
    '(fsub(dx2, l), dxy)' \
    '(dxy, fsub(dx2, l))'

mutate CATCH "else arm reuses the if arm's parameter binding" \
    'let (dy2, dx2, dxy) = (s0, s1, s2);' \
    'let (dx2, dy2, dxy) = (s0, s1, s2);'

mutate CATCH "count treated as unsigned (negative counts wrap)" \
    'let mut i: c_int = 0;
    while i < count {' \
    'let mut i: c_int = 0;
    while (i as u32) < (count as u32) {'

mutate CATCH "src stride 2 instead of 3" \
    'let inp = 3isize * i as isize;' \
    'let inp = 2isize * i as isize;'

mutate CATCH "dest stride 3 instead of 2" \
    'let out = 2isize * i as isize;' \
    'let out = 3isize * i as isize;'

mutate CATCH "dest written before src is fully read (breaks aliasing)" \
    'let s2 = src.offset(inp + 2).read_unaligned();

        let (d0, d1) = step(s0, s1, s2);

        let out = 2isize * i as isize;
        dest.offset(out).write_unaligned(d0);
        dest.offset(out + 1).write_unaligned(d1);' \
    'let out = 2isize * i as isize;
        dest.offset(out).write_unaligned(0);
        let s2 = src.offset(inp + 2).read_unaligned();
        let (d0, d1) = step(s0, s1, s2);
        dest.offset(out).write_unaligned(d0);
        dest.offset(out + 1).write_unaligned(d1);'

mutate CATCH "dx2*dx2 term replaced by dy2*dy2" \
    'sqd = fadd(sqd, fmul(dx2, dx2));' \
    'sqd = fadd(sqd, fmul(dy2, dy2));'

mutate CATCH "subtraction turned into an addition" \
    'let mut sqd = fsub(fmul(dy2, dy2), two_dx2_dy2);' \
    'let mut sqd = fadd(fmul(dy2, dy2), two_dx2_dy2);'

mutate CATCH "subtraction operands swapped" \
    'let mut sqd = fsub(fmul(dy2, dy2), two_dx2_dy2);' \
    'let mut sqd = fsub(two_dx2_dy2, fmul(dy2, dy2));'

mutate CATCH "4.0f constant becomes 2.0f" \
    'const FOUR: u32 = 0x4080_0000;' \
    'const FOUR: u32 = 0x4000_0000;'

mutate CATCH "0.5f constant becomes 2.0f" \
    'const HALF: u32 = 0x3f00_0000;' \
    'const HALF: u32 = 0x4000_0000;'

mutate CATCH "final subtraction operands swapped (lambda - dx2)" \
    '(fsub(dx2, l), dxy)
    } else {' \
    '(fsub(l, dx2), dxy)
    } else {'

mutate CATCH "is_nan_bits misses signaling NaNs" \
    '(x & 0x7f80_0000) == 0x7f80_0000 && (x & 0x007f_ffff) != 0' \
    '(x & 0x7fc0_0000) == 0x7fc0_0000 && (x & 0x007f_ffff) != 0'

mutate CATCH "loop bound off by one" \
    'while i < count {' \
    'while i < count - 1 {'

# ---------------------------------------------------------------------------
# Mutants that are PROVABLY EQUIVALENT through the public API.
# The justification for each is asserted against the C .so in
# tests/nan_masking.rs.
# ---------------------------------------------------------------------------

# `2.0f` is never NaN, so `mulss` returns quiet(dx2) exactly like
# `addss %xmm0,%xmm0`; and scaling an f32 by two is value-exact, overflow
# included. See nan_masking::two_times_x_equals_x_plus_x_including_payloads.
mutate EQUIVALENT "2*dx2 as a mulss against 2.0f instead of addss dx2,dx2" \
    'fmul(fadd(dx2, dx2), dy2)' \
    'fmul(fmul(0x4000_0000, dx2), dy2)'

# Commuting only matters when BOTH operands are NaN, i.e. dx2 and dy2 are both
# NaN, which happens only on the else arm with src[0] and src[1] both NaN -- and
# then dest[1] = subss(dx2, lambda) returns quiet(src[1]) regardless of lambda,
# and dest[0] = src[2] verbatim.
# See nan_masking::src1_nan_masks_every_intermediate_payload and
#     nan_masking::if_arm_never_sees_nan_in_dx2_or_dy2.
mutate EQUIVALENT "2*dx2*dy2 operand order swapped" \
    'fmul(fadd(dx2, dx2), dy2)' \
    'fmul(dy2, fadd(dx2, dx2))'

mutate EQUIVALENT "dy2+dx2 operand order swapped" \
    'fadd(fadd(dy2, dx2), fsqrt(clamped))' \
    'fadd(fadd(dx2, dy2), fsqrt(clamped))'

# 0.5f is never NaN, so only one operand can ever be NaN here.
mutate EQUIVALENT "0.5f * sum operand order swapped" \
    'fmul(HALF, fadd(fadd(dy2, dx2), fsqrt(clamped)))' \
    'fmul(fadd(fadd(dy2, dx2), fsqrt(clamped)), HALF)'

# Dead code: the inlined MAX(0, sqd) makes a negative sqrtf argument
# unreachable. See ERRORS.md row 23 / phase_c_errors::row23_*.
mutate EQUIVALENT "sqrt of a negative returns +qNaN (dead branch)" \
    'return INDEFINITE;
    }
    v.sqrt().to_bits()' \
    'return 0x7fc0_0000;
    }
    v.sqrt().to_bits()'

# Rust's `<` on f32 is already an ordered comparison (false for NaN), exactly
# like comiss+jbe. See nan_masking::rust_lt_is_ordered_like_comiss.
mutate EQUIVALENT "branch guard drops its redundant explicit NaN checks" \
    '!is_nan_bits(a) && !is_nan_bits(b) && f32::from_bits(a) < f32::from_bits(b)' \
    'f32::from_bits(a) < f32::from_bits(b)'

# `fsqrt`'s result is consumed only by `fadd(sum, root)`, which either discards
# a NaN `root` (sum already NaN) or returns `quiet(root)`. `quiet` is idempotent,
# so quieting inside `fsqrt` is unobservable.
# See nan_masking::computed_output_slot_is_always_quiet_when_nan.
mutate EQUIVALENT "sqrt does not quiet a NaN operand" \
    'if is_nan_bits(a) {
        return quiet(a);
    }' \
    'if is_nan_bits(a) {
        return a;
    }'

# `sqd < -0.0` is the same predicate as `sqd < +0.0`, and `sqrtf(-0.0) == -0.0`
# could only survive `(dy2 + dx2) + root` if `dy2 + dx2 == -0.0`, which forces
# `dy2 == dx2 == -0.0` and hence a non-negative `sqd` (clamp not taken).
# See nan_masking::clamp_zero_sign_is_unobservable.
mutate EQUIVALENT "clamp target becomes -0.0f instead of +0.0f" \
    'const ZERO: u32 = 0x0000_0000;' \
    'const ZERO: u32 = 0x8000_0000;'

# ---------------------------------------------------------------------------

restore
cargo build --offline --release >/dev/null 2>&1

echo
echo "$n_catch behavioural mutants caught, $n_equiv equivalent mutants correctly tolerated."
if [ "$fail" -eq 0 ]; then
    echo "MUTATION CHECK PASSED."
else
    echo "MUTATION CHECK FAILED."
fi
exit "$fail"
