//! Phase C — error / rejection-path differential tests, one test per
//! `ERRORS.md` row.
//!
//! `ldexp_q2` has **no error surface**: it is a total function over
//! `float x int32` with a single `return` statement, no asserts, no sentinels,
//! and no pointer/length/enum parameters (see `ERRORS.md` for the mechanical
//! grep proving this). These tests therefore cover the boundary and
//! implementation-defined conditions the C code's two branches can be driven
//! into, and each asserts:
//!
//!   1. **the differential requirement** — C and Rust return the *same* value,
//!      compared by raw bits (so `+0.0` vs `-0.0` and NaN sign/payload count as
//!      divergences, not as "both returned something float-ish"); and
//!   2. **the pinned C ground truth** — the exact value observed from the
//!      compiled C `.so` on x86-64/gcc, so this file also documents the
//!      concrete sentinel each condition produces rather than merely asserting
//!      the two sides agree.

mod common;

use common::*;

/// Assert C and Rust agree, and that C produced exactly `expected_c_bits`.
#[track_caller]
fn check_pinned(y: f32, exp_q2: i32, expected_c_bits: u32) {
    let im = impls();
    let cv = im.c(y, exp_q2);
    let rv = im.rust(y, exp_q2);
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "DIVERGENCE ldexp_q2(y=0x{:08x}, exp_q2={exp_q2}): C=0x{:08x} Rust=0x{:08x}",
        y.to_bits(),
        cv.to_bits(),
        rv.to_bits()
    );
    assert_eq!(
        cv.to_bits(),
        expected_c_bits,
        "C ground truth changed for ldexp_q2(y=0x{:08x}, exp_q2={exp_q2}): \
         got 0x{:08x}, ERRORS.md documents 0x{expected_c_bits:08x}",
        y.to_bits(),
        cv.to_bits()
    );
}

/// The bit pattern `mulss` produces for a signalling-NaN operand: quiet it by
/// setting mantissa bit 22, leaving sign and the rest of the payload alone.
fn quiet(bits: u32) -> u32 {
    bits | 0x0040_0000
}

fn is_snan(bits: u32) -> bool {
    let exp = (bits >> 23) & 0xFF;
    let mant = bits & 0x007F_FFFF;
    exp == 0xFF && mant != 0 && (mant & 0x0040_0000) == 0
}

// ===========================================================================
// E1 — exp_q2 == 0: scale is exactly 1.0f, so the function is the identity.
// ===========================================================================
#[test]
fn e1_exp_zero_is_identity() {
    let mut rng = Rng::new(0xE001);
    // Every special value: identity preserves bits, except sNaN gets quieted.
    for &bits in SPECIAL_Y_BITS {
        let expected = if is_snan(bits) { quiet(bits) } else { bits };
        check_pinned(f32::from_bits(bits), 0, expected);
    }
    // Randomized: any non-sNaN bit pattern must come back unchanged.
    let mut n = 0;
    for _ in 0..4000 {
        let bits = rng.next_u32();
        let expected = if is_snan(bits) { quiet(bits) } else { bits };
        check_pinned(f32::from_bits(bits), 0, expected);
        n += 1;
    }
    eprintln!("E1: {} identity cases OK", n + SPECIAL_Y_BITS.len());
}

// ===========================================================================
// E2 — exp_q2 == 120: the clamp boundary hit exactly. e == 120, and
// `exp_q2 -= e` yields 0, so exactly ONE trip (not two).
// ===========================================================================
#[test]
fn e2_clamp_boundary_exact() {
    check_pinned(1.0, 120, 0x3080_0000);
    check_pinned(-1.0, 120, 0xB080_0000);
    // Cross-check the "exactly one trip" claim: 120 must equal a single
    // 2^-30 scaling, i.e. differ from exp_q2 = 240 (two trips).
    let im = impls();
    assert_ne!(
        im.c(1.0, 120).to_bits(),
        im.c(1.0, 240).to_bits(),
        "exp_q2=120 must be one trip, 240 must be two"
    );
    let mut rng = Rng::new(0xE002);
    let cases = (0..2000).map(|_| (rng.next_f32_bits(), 120));
    check_all("E2 clamp boundary exp_q2=120", cases);
}

// ===========================================================================
// E3 — exp_q2 == 121: ONE STEP PAST the clamp, so e == 120 then a second trip
// with e == 1.
// ===========================================================================
#[test]
fn e3_one_past_clamp() {
    check_pinned(1.0, 121, 0x3057_44FD);
    let mut rng = Rng::new(0xE003);
    let cases = (0..2000).map(|_| (rng.next_f32_bits(), 121));
    check_all("E3 one past clamp exp_q2=121", cases);
}

// ===========================================================================
// E4 — exp_q2 == 119: one step BELOW the clamp. Single trip, residue 3,
// shift count 29 => scale 2.
// ===========================================================================
#[test]
fn e4_one_below_clamp() {
    check_pinned(1.0, 119, 0x3098_37F0);
    let mut rng = Rng::new(0xE004);
    let cases = (0..2000).map(|_| (rng.next_f32_bits(), 119));
    check_all("E4 one below clamp exp_q2=119", cases);
}

// ===========================================================================
// E5 — exp_q2 < 0: `e` is negative, so `e >> 2` is a NEGATIVE shift count and
// `(1 << 30) >> (e >> 2)` is undefined behaviour in C. gcc/x86-64 emits
// `sar %cl`, whose count the CPU masks to 5 bits, so the observable shift is
// by `(e >> 2) & 31`. Must not trap, and must match the Rust masking.
// ===========================================================================
#[test]
fn e5_negative_exp_ub_shift() {
    let mut rng = Rng::new(0xE005);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    // Exhaustive over more than four full 128-periods of the masking pattern.
    for e in -600..0 {
        for y in special_ys() {
            cases.push((y, e));
        }
        for _ in 0..4 {
            cases.push((rng.next_normal_f32(), e));
        }
    }
    check_all("E5 negative exp_q2 (UB masked shift)", cases);

    // Pin the masked-shift structure: k = (e>>2) & 31 is periodic with
    // period 128 in e, so exp_q2 = -1 and -129 and -257 must agree.
    let im = impls();
    for base in [-1i32, -2, -3, -4, -5, -33, -67] {
        let a = im.c(1.0, base).to_bits();
        for mult in 1..=4 {
            let e2 = base - 128 * mult;
            assert_eq!(
                im.c(1.0, e2).to_bits(),
                a,
                "masked shift must be 128-periodic in e: exp_q2={base} vs {e2}"
            );
            assert_eq!(im.rust(1.0, e2).to_bits(), a, "Rust must be 128-periodic too");
        }
    }
}

// ===========================================================================
// E6 — exp_q2 in {-1,-2,-3,-4}: e>>2 == -1, masked count == 31, so
// (1<<30)>>31 == 0. The scale ANNIHILATES y: the result is a signed zero for
// every finite y (and NaN for inf, covered by E10).
// ===========================================================================
#[test]
fn e6_scale_zero_annihilates() {
    // All four residues, both signs of y.
    for &e in EXP_SCALE_ZERO {
        check_pinned(1.0, e, 0x0000_0000);
        check_pinned(-1.0, e, 0x8000_0000);
        check_pinned(0.0, e, 0x0000_0000);
        check_pinned(-0.0, e, 0x8000_0000);
        check_pinned(f32::MAX, e, 0x0000_0000);
        check_pinned(f32::MIN, e, 0x8000_0000);
        check_pinned(f32::from_bits(0x0000_0001), e, 0x0000_0000);
        check_pinned(f32::from_bits(0x8000_0001), e, 0x8000_0000);
    }
    // Randomized: every FINITE y must map to zero carrying y's sign bit.
    let mut rng = Rng::new(0xE006);
    let mut n = 0;
    for _ in 0..4000 {
        let y = rng.next_normal_f32();
        let e = EXP_SCALE_ZERO[(rng.next_u32() as usize) % EXP_SCALE_ZERO.len()];
        check_pinned(y, e, y.to_bits() & 0x8000_0000);
        n += 1;
    }
    eprintln!("E6: {n} annihilation cases OK");
}

// ===========================================================================
// E7 / E8 — exp_q2 == INT_MIN and its neighbourhood. `e == INT_MIN`, residue
// 0, `e >> 2 == -536870912` whose low 5 bits are 0, so scale == 2^30 and the
// call is the IDENTITY. Also pins the `exp_q2 -= e` corner: INT_MIN - INT_MIN
// == 0 exactly, with no signed overflow, so the loop runs once.
// ===========================================================================
#[test]
fn e7_int_min() {
    // INT_MIN is divisible by 128, so it lands on the identity lattice.
    assert_eq!(i32::MIN % 128, 0, "premise: INT_MIN is on the 128-lattice");
    for &bits in SPECIAL_Y_BITS {
        let expected = if is_snan(bits) { quiet(bits) } else { bits };
        check_pinned(f32::from_bits(bits), i32::MIN, expected);
    }
    // E8: the neighbourhood INT_MIN+1 ..= INT_MIN+8 (non-zero residues).
    let mut rng = Rng::new(0xE007);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for d in 0..=8 {
        for y in special_ys() {
            cases.push((y, i32::MIN + d));
        }
        for _ in 0..64 {
            cases.push((rng.next_f32_bits(), i32::MIN + d));
        }
    }
    // and the top of the negative range generally
    for d in 0..256 {
        cases.push((rng.next_normal_f32(), i32::MIN + d));
    }
    check_all("E7/E8 INT_MIN neighbourhood", cases);
}

// ===========================================================================
// E9 — exp_q2 == INT_MAX: the maximum trip count, ceil(2147483647/120) ==
// 17,895,698 iterations of the do/while. Must terminate and agree.
// ===========================================================================
#[test]
fn e9_int_max() {
    assert_eq!(
        (i32::MAX as i64 + 119) / 120,
        17_895_698,
        "premise: INT_MAX drives 17,895,698 trips"
    );
    check_pinned(1.0, i32::MAX, 0x0000_0000);
    check_pinned(-1.0, i32::MAX, 0x8000_0000);
    check_pinned(0.0, i32::MAX, 0x0000_0000);
    check_pinned(-0.0, i32::MAX, 0x8000_0000);
    check_pinned(f32::MAX, i32::MAX, 0x0000_0000);
    check_pinned(f32::MIN, i32::MAX, 0x8000_0000);
    // inf survives every 2^-30 scaling; NaN propagates.
    check_pinned(f32::INFINITY, i32::MAX, 0x7F80_0000);
    check_pinned(f32::NEG_INFINITY, i32::MAX, 0xFF80_0000);
    check_pinned(f32::from_bits(0x7FC0_1234), i32::MAX, 0x7FC0_1234);
    eprintln!("E9: INT_MAX (17,895,698 trips) OK");
}

// ===========================================================================
// E10 — y == +/-inf with a zero scale: the IEEE-754 INVALID OPERATION
// `inf * 0`. x86 SSE returns the "QNaN indefinite" 0xFFC00000 (sign SET) for
// BOTH +inf and -inf. This is a genuinely different sentinel from the sNaN
// quieting in E12, so it is pinned separately.
// ===========================================================================
#[test]
fn e10_inf_times_zero_scale() {
    for &e in EXP_SCALE_ZERO {
        check_pinned(f32::INFINITY, e, 0xFFC0_0000);
        check_pinned(f32::NEG_INFINITY, e, 0xFFC0_0000);
    }
    // Sanity: with a NON-zero scale, inf stays inf (no invalid operation).
    check_pinned(f32::INFINITY, 0, 0x7F80_0000);
    check_pinned(f32::NEG_INFINITY, 0, 0xFF80_0000);
    check_pinned(f32::INFINITY, 120, 0x7F80_0000);
    check_pinned(f32::NEG_INFINITY, 120, 0xFF80_0000);
    eprintln!("E10: inf*0 -> 0xFFC00000 OK");
}

// ===========================================================================
// E11 — quiet NaN propagation with payload preservation.
// ===========================================================================
#[test]
fn e11_nan_propagation() {
    let mut exps: Vec<i32> = vec![0, 120, 119, 121];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.push(i32::MIN);

    // A quiet NaN passes through every scale regime with its bits intact:
    // qNaN * anything == that same qNaN on x86.
    for &bits in NAN_Y_BITS {
        assert!(!is_snan(bits), "NAN_Y_BITS must all be quiet");
        for &e in &exps {
            check_pinned(f32::from_bits(bits), e, bits);
        }
    }
    // Randomized quiet NaNs (random payloads, both signs).
    let mut rng = Rng::new(0xE011);
    let mut n = 0;
    for _ in 0..3000 {
        let payload = rng.next_u32() & 0x007F_FFFF;
        let sign = (rng.next_u32() & 1) << 31;
        let bits = sign | 0x7F80_0000 | 0x0040_0000 | payload; // quiet by construction
        let e = rng.range_i32(-600, 600);
        check_pinned(f32::from_bits(bits), e, bits);
        n += 1;
    }
    eprintln!("E11: {n} qNaN propagation cases OK");
}

// ===========================================================================
// E12 — signalling NaN across the FFI boundary: quieted by setting mantissa
// bit 22, sign and remaining payload preserved.
// ===========================================================================
#[test]
fn e12_snan_quieting() {
    check_pinned(f32::from_bits(0x7FA0_0000), 0, 0x7FE0_0000);
    check_pinned(f32::from_bits(0xFFA0_0000), 0, 0xFFE0_0000);
    check_pinned(f32::from_bits(0x7F80_0001), 0, 0x7FC0_0001);
    // A zero scale does NOT turn an sNaN into the indefinite: the NaN operand
    // wins over the invalid-operation default.
    check_pinned(f32::from_bits(0x7FA0_0000), -1, 0x7FE0_0000);
    check_pinned(f32::from_bits(0xFFA0_0000), -1, 0xFFE0_0000);
    check_pinned(f32::from_bits(0x7F80_0001), -1, 0x7FC0_0001);

    let mut exps: Vec<i32> = vec![0, 119, 120, 121, i32::MIN];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    for &bits in SNAN_Y_BITS {
        assert!(is_snan(bits), "SNAN_Y_BITS must all be signalling");
        for &e in &exps {
            check_pinned(f32::from_bits(bits), e, quiet(bits));
        }
    }
    // Randomized sNaNs.
    let mut rng = Rng::new(0xE012);
    let mut n = 0;
    for _ in 0..3000 {
        let payload = (rng.next_u32() & 0x003F_FFFF).max(1); // bit 22 clear, non-zero
        let sign = (rng.next_u32() & 1) << 31;
        let bits = sign | 0x7F80_0000 | payload;
        assert!(is_snan(bits));
        let e = rng.range_i32(-600, 600);
        check_pinned(f32::from_bits(bits), e, quiet(bits));
        n += 1;
    }
    eprintln!("E12: {n} sNaN quieting cases OK");
}

// ===========================================================================
// E13 — signed zero: sign(result) == sign(y) ^ sign(scale); every scale is
// non-negative, so y's sign is preserved through all regimes.
// ===========================================================================
#[test]
fn e13_signed_zero() {
    let mut exps: Vec<i32> = vec![0, 119, 120, 121, i32::MIN, i32::MAX];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(-200..=200);
    for &e in &exps {
        check_pinned(0.0, e, 0x0000_0000);
        check_pinned(-0.0, e, 0x8000_0000);
    }
    eprintln!("E13: signed zero across {} exponents OK", exps.len());
}

// ===========================================================================
// E14 — subnormal y with a scale < 1: gradual underflow to zero.
// ===========================================================================
#[test]
fn e14_subnormal_underflow() {
    // NOTE: the total multiplier is `frac[e&3] * 2^(30-k)`, so a *scale* of 1
    // (k == 30) still means a multiplier of ~2^-30 -- only k == 0 is the
    // identity. Both k == 30 and k == 29.. therefore flush subnormals to zero.
    //
    // Smallest positive subnormal, multiplier 2^-30 => flushes to signed zero.
    check_pinned(f32::from_bits(0x0000_0001), 120, 0x0000_0000);
    check_pinned(f32::from_bits(0x8000_0001), 120, 0x8000_0000);
    check_pinned(f32::from_bits(0x0000_0001), -8, 0x0000_0000);
    check_pinned(f32::from_bits(0x8000_0001), -8, 0x8000_0000);
    // Even the LARGEST subnormal underflows all the way to zero at 2^-30.
    check_pinned(f32::from_bits(0x007F_FFFF), 120, 0x0000_0000);
    check_pinned(f32::from_bits(0x807F_FFFF), -8, 0x8000_0000);
    // The identity (k == 0) leaves subnormals untouched: exp_q2 == 0 and the
    // negative 128-lattice.
    check_pinned(f32::from_bits(0x0000_0001), 0, 0x0000_0001);
    check_pinned(f32::from_bits(0x007F_FFFF), 0, 0x007F_FFFF);
    for &e in EXP_NEG_IDENTITY {
        check_pinned(f32::from_bits(0x0000_0001), e, 0x0000_0001);
        check_pinned(f32::from_bits(0x8000_0001), e, 0x8000_0001);
        check_pinned(f32::from_bits(0x007F_FFFF), e, 0x007F_FFFF);
    }

    let mut rng = Rng::new(0xE014);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for _ in 0..4000 {
        cases.push((rng.next_subnormal_f32(), rng.range_i32(-600, 600)));
    }
    for e in -300..=300 {
        cases.push((f32::from_bits(0x0000_0001), e));
        cases.push((f32::from_bits(0x8000_0001), e));
        cases.push((f32::from_bits(0x007F_FFFF), e));
        cases.push((f32::from_bits(0x807F_FFFF), e));
    }
    check_all("E14 subnormal gradual underflow", cases);
}

// ===========================================================================
// E15 — extreme finite y. Every scale is <= 1.0, so no input can overflow to
// infinity; verified against C rather than assumed.
// ===========================================================================
#[test]
fn e15_extreme_finite() {
    check_pinned(f32::MAX, 0, 0x7F7F_FFFF);
    check_pinned(f32::MIN, 0, 0xFF7F_FFFF);
    check_pinned(f32::MIN_POSITIVE, 0, 0x0080_0000);

    let im = impls();
    let extremes = [f32::MAX, f32::MIN, f32::MIN_POSITIVE, -f32::MIN_POSITIVE];
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for &y in &extremes {
        for e in -600..=600 {
            cases.push((y, e));
            // No scale exceeds 1.0, so a finite y can never become infinite.
            let cv = im.c(y, e);
            assert!(
                cv.is_finite(),
                "finite y=0x{:08x} became non-finite at exp_q2={e}: 0x{:08x}",
                y.to_bits(),
                cv.to_bits()
            );
        }
    }
    check_all("E15 extreme finite y", cases);
}

// ===========================================================================
// E16 — negative-e residue classes: `e & 3` on a negative `e` indexes
// g_expfrac with two's-complement low bits (e.g. -1 & 3 == 3), which stays in
// bounds 0..3. Confirms Rust's `(e & 3) as usize` picks the same element.
// ===========================================================================
#[test]
fn e16_negative_residue_classes() {
    // For a fixed shift count, the four residues must select four DIFFERENT
    // g_expfrac entries; if Rust computed the index differently (e.g. via
    // rem_euclid on a signed value, or by clamping) this would collapse.
    let im = impls();
    let mut seen = std::collections::BTreeSet::new();
    for e in [-8i32, -7, -6, -5] {
        // all k == 30 (scale 1), residues 0,1,2,3
        assert_eq!((e >> 2) & 31, 30);
        let bits = im.c(1.0, e).to_bits();
        assert_eq!(im.rust(1.0, e).to_bits(), bits, "residue mismatch at e={e}");
        seen.insert(bits);
    }
    assert_eq!(
        seen.len(),
        4,
        "the four negative residues must select four distinct g_expfrac entries, got {seen:?}"
    );

    // Confirm they equal the corresponding POSITIVE-e residues at the same
    // scale (exp_q2 = 120 has k == 30, residue 0).
    assert_eq!(
        im.c(1.0, -8).to_bits(),
        im.c(1.0, 120).to_bits(),
        "e=-8 (r=0,k=30) must equal e=120 (r=0,k=30)"
    );
    assert_eq!(im.rust(1.0, -8).to_bits(), im.rust(1.0, 120).to_bits());

    // Exhaustive negative residue sweep.
    let mut rng = Rng::new(0xE016);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for e in -520..0 {
        for y in special_ys() {
            cases.push((y, e));
        }
        cases.push((rng.next_normal_f32(), e));
    }
    check_all("E16 negative residue classes", cases);
}

// ===========================================================================
// E17 — exhaustive sweep of the small-exp_q2 neighbourhood crossed with every
// special y. Catches any off-by-one in the clamp, the residue, or the shift
// masking.
// ===========================================================================
#[test]
fn e17_exhaustive_small_exp_all_special_y() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for e in -1000..=1000 {
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("E17 exhaustive exp_q2 -1000..=1000 x all special y", cases);
}

// ===========================================================================
// E18 — the "out-of-range value across the FFI boundary" class. `int exp_q2`
// has no invalid value (all 2^32 are legal input, and the API declares no
// enum), so this row fuzzes the entire int32 domain, stratified so trip counts
// stay bounded.
// ===========================================================================
#[test]
fn e18_full_int_range_random() {
    let mut rng = Rng::new(0xE018);
    let mut cases: Vec<(f32, i32)> = Vec::new();

    // Unrestricted negatives: always one trip, so the full range is cheap.
    for _ in 0..20000 {
        let e = rng.next_i32() | i32::MIN; // force the sign bit => any negative
        cases.push((rng.next_f32_bits(), e));
    }
    // Bounded positives.
    for _ in 0..5000 {
        cases.push((rng.next_f32_bits(), rng.range_i32(0, 200_000)));
    }
    // Powers of two and their neighbours across the whole domain.
    for shift in 0..31u32 {
        let p = 1i32 << shift;
        for d in [-1i32, 0, 1] {
            let e = p.wrapping_add(d);
            // keep the trip count bounded for large positives
            if e <= 2_000_000 {
                cases.push((rng.next_normal_f32(), e));
            }
            cases.push((rng.next_normal_f32(), -e));
        }
    }
    cases.push((1.0, i32::MIN));
    cases.push((1.0, i32::MAX));
    check_all("E18 full int32 range (no invalid value exists)", cases);
}

// ===========================================================================
// E-N/A — the generic boundary classes that structurally cannot apply.
// Asserted mechanically against the C header so the ERRORS.md "N/A" rows stay
// honest if the API ever grows a pointer, length, or enum parameter.
// ===========================================================================
#[test]
fn e_na_no_pointer_length_or_enum_parameters() {
    let header = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/include/lib.h"
    ))
    .expect("c_src/include/lib.h must be readable");

    // No pointer parameter => no null-pointer boundary to test.
    assert!(
        !header.contains('*'),
        "the C API grew a pointer parameter; ERRORS.md's null-pointer N/A row \
         is no longer valid:\n{header}"
    );
    // No enum => no out-of-range enum value to pass across the FFI boundary.
    assert!(
        !header.contains("enum"),
        "the C API grew an enum; ERRORS.md's out-of-range-enum N/A row is no \
         longer valid:\n{header}"
    );
    // No size/length/count parameter => no zero/oversized length boundary.
    for kw in ["size_t", "len", "count", "size", "n_", "num"] {
        assert!(
            !header.to_lowercase().contains(kw),
            "the C API grew a `{kw}` parameter; ERRORS.md's length N/A row is \
             no longer valid:\n{header}"
        );
    }
    // And the whole C implementation still has no rejection mechanism.
    let src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/src/lib.c")).unwrap();
    for kw in ["assert", "errno", "abort(", "exit(", "NULL", "#ifdef", "#if "] {
        assert!(
            !src.contains(kw),
            "c_src/src/lib.c grew a `{kw}`; the ERRORS.md empty-error-surface \
             derivation must be redone"
        );
    }
    assert_eq!(
        src.matches("return").count(),
        1,
        "c_src/src/lib.c must still have exactly one `return` (the success path)"
    );
    eprintln!("E-N/A: pointer/length/enum boundary classes confirmed inapplicable");
}
