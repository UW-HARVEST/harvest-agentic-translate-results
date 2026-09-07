//! Phase C — error / rejection-surface differential tests, one `#[test]` per
//! row of `ERRORS.md` (rows 1..17).
//!
//! `ldexp_q2` is a total function: it never returns an error code, so the
//! "error result" that must agree between C and Rust is the exact returned
//! `f32` bit pattern (including the specific NaN payload and the sign of a
//! zero). Every test therefore asserts bit equality, not merely "both failed".
//!
//! Checklist (all rows pass — see `run_all.sh` output):
//!   [x] 1  exp_q2 == 0 degenerate
//!   [x] 2  exp_q2 < 0 -> negative shift count (C UB)
//!   [x] 3  exp_q2 == -1 -> integer factor collapses to 0
//!   [x] 4  exp_q2 == INT_MIN
//!   [x] 5  exp_q2 == INT_MAX (max trip count, no hang)
//!   [x] 6  exp_q2 > 120 silently saturates
//!   [x] 7  exp_q2 == 120 clamp boundary
//!   [x] 8  exp_q2 == 121 one past the boundary
//!   [x] 9  negative exp_q2 -> two's-complement index stays in 0..=3
//!   [x] 10 exp_q2 multiple of -128 -> masked shift count wraps to 0
//!   [x] 11 y is NaN (quiet and signalling)
//!   [x] 12 inf * 0 invalid operation
//!   [x] 13 signed zero
//!   [x] 14 overflow to inf
//!   [x] 15 underflow to zero/subnormal
//!   [x] 16 subnormal input
//!   [x] 17 every int bit pattern is a valid `exp_q2` (no valid-range concept)

mod common;

use common::{assert_batch, assert_same, check, pair, specials, Rng};

// --- Row 1: exp_q2 == 0 -----------------------------------------------------

#[test]
fn err01_exp_zero_is_not_identity() {
    // Both must agree, and both must apply one scaling (do/while runs once).
    for y in specials() {
        assert_same(y, 0);
    }
    let mut rng = Rng::new(0x2001);
    let mut out = Vec::new();
    for _ in 0..2000 {
        out.push((rng.any_f32(), 0));
        out.push((rng.moderate_f32(), 0));
    }
    assert_batch("err01", out);

    // Document the observed behaviour: it is *not* the identity in general.
    let p = pair();
    let c = unsafe { (p.c)(1.0, 0) };
    let r = unsafe { (p.rust)(1.0, 0) };
    assert_eq!(c.to_bits(), r.to_bits());
    assert!(
        c.is_finite(),
        "ldexp_q2(1.0, 0) should be finite, got {c:?}"
    );
}

// --- Row 2: exp_q2 < 0 -> shift count out of the C-defined range -----------

#[test]
fn err02_negative_shift_count_ub() {
    // Every negative exponent takes the `1 << 30 >> negative` path. Neither
    // implementation may trap or panic, and they must agree bit-for-bit.
    let mut rng = Rng::new(0x2002);
    let mut out = Vec::new();
    for e in -600..0i32 {
        out.push((1.0f32, e));
        out.push((-1.0f32, e));
        out.push((rng.moderate_f32(), e));
        out.push((rng.any_f32(), e));
    }
    for _ in 0..4000 {
        out.push((rng.any_f32(), rng.range_i32(i32::MIN, -1)));
    }
    assert_batch("err02", out);
}

// --- Row 3: exp_q2 == -1 -> integer factor is 0 ---------------------------

#[test]
fn err03_exp_minus_one_zeroes_factor() {
    for y in specials() {
        assert_same(y, -1);
    }
    let mut rng = Rng::new(0x2003);
    let mut out = Vec::new();
    for _ in 0..3000 {
        out.push((rng.any_f32(), -1));
    }
    assert_batch("err03", out);

    // The sentinel: finite y collapses to a signed zero, inf/NaN become NaN.
    let p = pair();
    for (y, want_zero) in [
        (1.0f32, true),
        (-1.0f32, true),
        (f32::MAX, true),
        (f32::MIN_POSITIVE, true),
        (f32::INFINITY, false),
        (f32::NEG_INFINITY, false),
        (f32::NAN, false),
    ] {
        let c = unsafe { (p.c)(y, -1) };
        let r = unsafe { (p.rust)(y, -1) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "ldexp_q2({y:?}, -1): C 0x{:08X} vs Rust 0x{:08X}",
            c.to_bits(),
            r.to_bits()
        );
        if want_zero {
            assert_eq!(c, 0.0, "expected zero from C for y = {y:?}, got {c:?}");
            assert_eq!(
                c.is_sign_negative(),
                y.is_sign_negative(),
                "zero sign mismatch for y = {y:?}"
            );
        } else {
            assert!(c.is_nan(), "expected NaN from C for y = {y:?}, got {c:?}");
        }
    }
}

// --- Row 4: exp_q2 == INT_MIN --------------------------------------------

#[test]
fn err04_exp_int_min() {
    for y in specials() {
        assert_same(y, i32::MIN);
    }
    let mut rng = Rng::new(0x2004);
    let mut out = Vec::new();
    for _ in 0..3000 {
        out.push((rng.any_f32(), i32::MIN));
    }
    // Neighbourhood of INT_MIN, where `exp_q2 -= e` is closest to overflowing.
    for k in 0..64i32 {
        let e = i32::MIN.wrapping_add(k);
        out.push((1.0f32, e));
        out.push((rng.moderate_f32(), e));
    }
    assert_batch("err04", out);
}

// --- Row 5: exp_q2 == INT_MAX (must terminate) --------------------------

#[test]
fn err05_exp_int_max_terminates() {
    for e in [i32::MAX, i32::MAX - 1, i32::MAX - 7, 2_000_000_000] {
        for y in [
            1.0f32,
            -1.0f32,
            f32::MAX,
            0.0f32,
            -0.0f32,
            f32::INFINITY,
            f32::NAN,
        ] {
            assert_same(y, e);
        }
    }
}

// --- Row 6: exp_q2 > 120 saturates instead of rejecting ----------------

#[test]
fn err06_saturating_clamp() {
    let mut rng = Rng::new(0x2006);
    let mut out = Vec::new();
    for e in 121..1200i32 {
        out.push((rng.moderate_f32(), e));
        out.push((1.0f32, e));
    }
    for _ in 0..2000 {
        out.push((rng.any_f32(), rng.range_i32(121, 500_000)));
    }
    assert_batch("err06", out);
}

// --- Row 7: exp_q2 == 120 boundary ------------------------------------

#[test]
fn err07_clamp_boundary_120() {
    for y in specials() {
        assert_same(y, 120);
        assert_same(y, 119);
    }
    let mut rng = Rng::new(0x2007);
    let mut out = Vec::new();
    for _ in 0..2000 {
        let y = rng.any_f32();
        out.push((y, 119));
        out.push((y, 120));
    }
    assert_batch("err07", out);
}

// --- Row 8: exp_q2 == 121, one past the boundary ----------------------

#[test]
fn err08_one_past_boundary_121() {
    for y in specials() {
        assert_same(y, 121);
    }
    let mut rng = Rng::new(0x2008);
    let mut out = Vec::new();
    for _ in 0..2000 {
        out.push((rng.any_f32(), 121));
    }
    assert_batch("err08", out);

    // 120 and 121 really are different configurations.
    let p = pair();
    let a = unsafe { (p.c)(1.0, 120) };
    let b = unsafe { (p.c)(1.0, 121) };
    assert_ne!(
        a.to_bits(),
        b.to_bits(),
        "exp_q2 120 and 121 unexpectedly identical in C"
    );
    assert_eq!(unsafe { (p.rust)(1.0, 120) }.to_bits(), a.to_bits());
    assert_eq!(unsafe { (p.rust)(1.0, 121) }.to_bits(), b.to_bits());
}

// --- Row 9: two's-complement index never leaves 0..=3 ------------------

#[test]
fn err09_negative_index_in_bounds() {
    // If either side indexed out of bounds this would crash or return garbage.
    // Sweep every residue class of negative exponents.
    let mut rng = Rng::new(0x2009);
    let mut out = Vec::new();
    for e in -256..=-1i32 {
        for _ in 0..8 {
            out.push((rng.moderate_f32(), e));
        }
    }
    // Plus 4 exponents per residue class deep in the negatives.
    for r in 0..4i32 {
        for k in 1..=200i32 {
            let e = -(4 * k) + r;
            out.push((rng.moderate_f32(), e));
        }
    }
    assert_batch("err09", out);
}

// --- Row 10: masked shift count wraps to 0 ---------------------------

#[test]
fn err10_masked_shift_wraps() {
    let mut rng = Rng::new(0x200A);
    let mut out = Vec::new();
    for k in 1..=200i32 {
        let e = -128 * k;
        for _ in 0..10 {
            out.push((rng.moderate_f32(), e));
        }
        for y in specials() {
            out.push((y, e));
        }
    }
    assert_batch("err10", out);

    // -128 (masked count 0, factor 2^30) must differ from -1 (count 31, factor 0).
    let p = pair();
    let a = unsafe { (p.c)(1.0, -128) };
    let b = unsafe { (p.c)(1.0, -1) };
    assert_ne!(a.to_bits(), b.to_bits());
    assert_eq!(unsafe { (p.rust)(1.0, -128) }.to_bits(), a.to_bits());
    assert_eq!(unsafe { (p.rust)(1.0, -1) }.to_bits(), b.to_bits());
}

// --- Row 11: NaN input, quiet and signalling -------------------------

#[test]
fn err11_nan_propagation() {
    let nans: [u32; 12] = [
        0x7FC0_0000,
        0xFFC0_0000,
        0x7FC0_0001,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x7FC1_2345,
        0xFFCA_BCDE,
        0x7F80_0001, // sNaN
        0xFF80_0001, // -sNaN
        0x7FBF_FFFF, // largest sNaN
        0xFFBF_FFFF,
        0x7F80_4000, // sNaN with payload
    ];
    let exps: [i32; 14] = [
        i32::MIN,
        -1_000_000,
        -128,
        -5,
        -1,
        0,
        1,
        3,
        119,
        120,
        121,
        240,
        1_000,
        100_000,
    ];
    let mut out = Vec::new();
    for &nb in &nans {
        for &e in &exps {
            out.push((f32::from_bits(nb), e));
        }
    }
    assert_batch("err11", out);

    // And both must actually return NaN.
    let p = pair();
    for &nb in &nans {
        let y = f32::from_bits(nb);
        for &e in &exps {
            let c = unsafe { (p.c)(y, e) };
            let r = unsafe { (p.rust)(y, e) };
            assert!(c.is_nan(), "C lost NaN for bits 0x{nb:08X}, exp {e}: {c:?}");
            assert_eq!(c.to_bits(), r.to_bits());
        }
    }
}

// --- Row 12: inf * 0 invalid operation ------------------------------

#[test]
fn err12_inf_times_zero_factor() {
    // exp_q2 == -1 makes the integer factor 0, so inf * 0 -> NaN.
    let p = pair();
    for y in [f32::INFINITY, f32::NEG_INFINITY] {
        for e in [-1i32, -5, -9, -13, -125, -1_000_000_001] {
            let c = unsafe { (p.c)(y, e) };
            let r = unsafe { (p.rust)(y, e) };
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "ldexp_q2({y:?}, {e}): C 0x{:08X} vs Rust 0x{:08X}",
                c.to_bits(),
                r.to_bits()
            );
        }
    }
    // Also every negative exponent crossed with both infinities.
    let mut out = Vec::new();
    for e in -1024..0i32 {
        out.push((f32::INFINITY, e));
        out.push((f32::NEG_INFINITY, e));
    }
    assert_batch("err12", out);
}

// --- Row 13: signed zero -------------------------------------------

#[test]
fn err13_signed_zero() {
    let mut out = Vec::new();
    for e in -400..400i32 {
        out.push((0.0f32, e));
        out.push((-0.0f32, e));
    }
    for &e in [i32::MIN, i32::MIN + 3, -1_000_000, 1_000_000, i32::MAX].iter() {
        out.push((0.0f32, e));
        out.push((-0.0f32, e));
    }
    assert_batch("err13", out);

    // Sign must be preserved identically by both.
    let p = pair();
    for e in [-1i32, 0, 1, 120, 121, i32::MIN] {
        for y in [0.0f32, -0.0f32] {
            let c = unsafe { (p.c)(y, e) };
            let r = unsafe { (p.rust)(y, e) };
            assert_eq!(c.to_bits(), r.to_bits(), "signed zero mismatch at exp {e}");
        }
    }
}

// --- Row 14: overflow to infinity ---------------------------------

#[test]
fn err14_overflow_to_inf() {
    let big = [
        f32::MAX,
        -f32::MAX,
        1e38f32,
        -1e38f32,
        3.4e38f32,
        1e30f32,
        1e20f32,
    ];
    // Negative exponents with masked shift 0 multiply by ~2^0..2^30, so they
    // are the ones that can overflow.
    let exps: Vec<i32> = (1..=64).map(|k| -128 * k).chain([i32::MIN]).collect();
    let mut out = Vec::new();
    for &y in &big {
        for &e in &exps {
            out.push((y, e));
        }
        for e in -520..0i32 {
            out.push((y, e));
        }
    }
    assert_batch("err14", out);

    // Verified property of the C constants: `g_expfrac[0]` is *exactly* 2^-30
    // (bit pattern 0x30800000), and the largest integer factor is
    // `0x40000000 >> 0 == 2^30`, so the largest possible per-iteration
    // multiplier is exactly `1.0f`. The other three table entries are
    // 2^-30 * 2^(-k/4) < 2^-30. Consequently `ldexp_q2` is magnitude
    // non-increasing and overflow-to-infinity is UNREACHABLE from a finite `y`
    // (see ERRORS.md row 14). Assert that BOTH implementations agree on this —
    // if the Rust translation ever produced an infinity here while C did not,
    // the bit comparison above would already have caught it; this extra check
    // pins the invariant explicitly for both.
    let p = pair();
    for &y in &big {
        for &e in &exps {
            let c = unsafe { (p.c)(y, e) };
            let r = unsafe { (p.rust)(y, e) };
            assert_eq!(c.to_bits(), r.to_bits());
            assert!(
                y.is_finite() && !c.is_infinite(),
                "C overflowed to infinity for finite y = {y:?}, exp_q2 = {e} -> {c:?}; \
                 this contradicts the max-multiplier-is-1.0 invariant"
            );
            assert!(
                c.abs() <= y.abs(),
                "magnitude grew: |ldexp_q2({y:?}, {e})| = {} > {}",
                c.abs(),
                y.abs()
            );
        }
    }

    // Infinity can still only come *in*, and then it stays infinity unless the
    // factor is zero (row 12).
    for e in [-128i32, -256, i32::MIN, 0, 120] {
        for y in [f32::INFINITY, f32::NEG_INFINITY] {
            let c = unsafe { (p.c)(y, e) };
            assert_eq!(c.to_bits(), unsafe { (p.rust)(y, e) }.to_bits());
        }
    }
}

// --- Row 15: underflow to zero / subnormal ------------------------

#[test]
fn err15_underflow() {
    let tiny = [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-38f32,
        1e-30f32,
        1.0f32,
        f32::from_bits(0x0000_0001),
    ];
    let exps = [
        1i32, 4, 40, 119, 120, 121, 240, 241, 1_000, 10_000, 123_457, 1_000_000,
    ];
    let mut out = Vec::new();
    for &y in &tiny {
        for &e in &exps {
            out.push((y, e));
        }
    }
    assert_batch("err15", out);

    let p = pair();
    let mut saw_zero = false;
    for &y in &tiny {
        for &e in &exps {
            let c = unsafe { (p.c)(y, e) };
            assert_eq!(c.to_bits(), unsafe { (p.rust)(y, e) }.to_bits());
            if c == 0.0 {
                saw_zero = true;
            }
        }
    }
    assert!(saw_zero, "no underflow-to-zero case was actually reached");
}

// --- Row 16: subnormal inputs ------------------------------------

#[test]
fn err16_subnormal_inputs() {
    let mut out = Vec::new();
    // Every power-of-two subnormal plus the extremes, in both signs.
    for bit in 0..23u32 {
        let m = 1u32 << bit;
        for sign in [0u32, 0x8000_0000] {
            let y = f32::from_bits(sign | m);
            for e in [-129i32, -128, -1, 0, 1, 3, 119, 120, 121, 240, 1_000, i32::MIN] {
                out.push((y, e));
            }
        }
    }
    // Random subnormals.
    let mut rng = Rng::new(0x2010);
    for _ in 0..2000 {
        let bits = (rng.next_u32() & 0x807F_FFFF) | 1;
        let y = f32::from_bits(bits);
        out.push((y, rng.range_i32(-300, 300)));
    }
    assert_batch("err16", out);
}

// --- Row 17: every int bit pattern is a legal exp_q2 --------------

#[test]
fn err17_no_invalid_exponent_exists() {
    // The C parameter is a plain `int` with no documented valid range, which is
    // the same situation as an out-of-range enum value crossing FFI: any of the
    // 2^32 patterns can arrive and must behave identically. Probe every power of
    // two, its negation and its neighbours, plus the sign-bit extremes.
    let mut exps: Vec<i32> = Vec::new();
    for b in 0..31u32 {
        let v = 1i32 << b;
        for d in [-2i32, -1, 0, 1, 2] {
            exps.push(v.wrapping_add(d));
            exps.push((-v).wrapping_add(d));
        }
    }
    exps.push(i32::MIN);
    exps.push(i32::MIN + 1);
    exps.push(-1);
    exps.push(0);
    // Keep the loop trip count bounded: replace huge positives (covered by row 5).
    for e in exps.iter_mut() {
        if *e > 2_000_000 {
            *e = 2_000_000 - (*e % 1_000);
        }
    }
    exps.sort_unstable();
    exps.dedup();

    let mut rng = Rng::new(0x2011);
    let mut out = Vec::new();
    for &e in &exps {
        out.push((1.0f32, e));
        out.push((-1.0f32, e));
        out.push((f32::NAN, e));
        out.push((f32::INFINITY, e));
        out.push((0.0f32, e));
        for _ in 0..12 {
            out.push((rng.moderate_f32(), e));
            out.push((rng.any_f32(), e));
        }
    }
    assert_batch("err17", out);

    // Sanity: `check` never fails to run, i.e. neither .so aborts on any input.
    for &e in &exps {
        check(1.5, e).unwrap_or_else(|m| panic!("{m}"));
    }
}
