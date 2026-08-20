//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Every test calls **both** shared objects
//! through their `extern "C"` exports and asserts bit-identical behaviour.
//!
//! `tfm` returns `void` and has no error code, so a "rejection" is observable
//! only as (a) `dest` being left untouched, (b) a specific arm being taken, or
//! (c) a specific IEEE-754 result (NaN payload / infinity) landing in `dest`.
//! Each test asserts the strongest available form.

mod common;
use common::*;

// ===========================================================================
// Rows 1-3 — the loop guard `i < count` rejects the whole call.
// ===========================================================================

/// ERRORS.md row 1 — `count == 0`: `dest` untouched.
#[test]
fn row01_count_zero_writes_nothing() {
    let src = vec![0x3f80_0000u32, 0x4000_0000, 0x4040_0000];
    diff("row01 count=0", &src, 0, 2);
    diff("row01 count=0 wide-dest", &src, 0, 64);
    // Non-trivial / hostile inputs must still be ignored.
    let mut rng = Rng::new(SEED ^ 0x01);
    for i in 0..64 {
        let s: Vec<u32> = (0..12).map(|_| rng.pool_f32()).collect();
        diff(&format!("row01 count=0 #{i}"), &s, 0, 8);
    }
}

/// ERRORS.md row 2 — `count < 0`: `dest` untouched, no unsigned wraparound.
#[test]
fn row02_count_negative_writes_nothing() {
    let mut rng = Rng::new(SEED ^ 0x02);
    let src: Vec<u32> = (0..12).map(|_| rng.pool_f32()).collect();
    for &count in &[-1i32, -2, -3, -7, -128, -1000, -65536, -0x0100_0000] {
        diff(&format!("row02 count={count}"), &src, count, 8);
    }
    for i in 0..256 {
        let c = -1i32 - (rng.next_u32() >> 1) as i32;
        diff(&format!("row02 random negative #{i} count={c}"), &src, c, 8);
    }
}

/// ERRORS.md row 3 — `count == INT_MIN` (and neighbours).
#[test]
fn row03_count_int_min_writes_nothing() {
    let src = vec![0x7f80_0000u32, 0xff80_0000, 0x7fc0_1234];
    for &count in &[i32::MIN, i32::MIN + 1, i32::MIN + 2, -i32::MAX] {
        diff(&format!("row03 count={count}"), &src, count, 4);
    }
}

// ===========================================================================
// Rows 4-6 — null pointers with a non-positive count are never dereferenced.
// ===========================================================================

/// ERRORS.md row 4 — `dest == NULL && src == NULL`, `count <= 0`.
#[test]
fn row04_both_null_non_positive_count() {
    for &count in &[0i32, -1, -5, i32::MIN] {
        diff_null(&format!("row04 both-null count={count}"), true, true, count);
    }
}

/// ERRORS.md row 5 — `src == NULL`, `dest` valid, `count <= 0`.
#[test]
fn row05_src_null_non_positive_count() {
    for &count in &[0i32, -1, -9999, i32::MIN] {
        diff_null(&format!("row05 src-null count={count}"), false, true, count);
    }
}

/// ERRORS.md row 6 — `dest == NULL`, `src` valid, `count <= 0`.
#[test]
fn row06_dest_null_non_positive_count() {
    for &count in &[0i32, -1, -42, i32::MIN] {
        diff_null(&format!("row06 dest-null count={count}"), true, false, count);
    }
}

// ===========================================================================
// Rows 7-12 — the branch guard `src[0] < src[1]` rejecting into the else arm.
// ===========================================================================

/// ERRORS.md row 7 — guard false because `src[0] > src[1]`.
#[test]
fn row07_guard_false_greater() {
    let mut rng = Rng::new(SEED ^ 0x07);
    let mut n = 0;
    for i in 0..4 * SAMPLES {
        let a = rng.normal_f32();
        let b = rng.normal_f32();
        let (lo, hi) = if f32::from_bits(a) < f32::from_bits(b) {
            (a, b)
        } else {
            (b, a)
        };
        if f32::from_bits(lo) >= f32::from_bits(hi) {
            continue; // equal or unordered; other rows cover those
        }
        let s = [hi, lo, rng.pool_f32()];
        let t = trace(s[0], s[1], s[2]);
        assert!(!t.arm_if, "row07: expected the else arm");
        diff(&format!("row07 #{i}"), &s, 1, 2);
        n += 1;
    }
    assert!(n >= SAMPLES, "row07: only {n} samples exercised the else arm");
}

/// ERRORS.md row 8 — guard false because `src[0] == src[1]` (`<` is strict).
#[test]
fn row08_guard_false_equal() {
    let mut rng = Rng::new(SEED ^ 0x08);
    let mut n = 0;
    for i in 0..2 * SAMPLES {
        let v = rng.pool_f32();
        if f32::from_bits(v).is_nan() {
            continue; // NaN equality is row 9-11
        }
        let s = [v, v, rng.pool_f32()];
        assert!(!trace(s[0], s[1], s[2]).arm_if, "row08: expected the else arm");
        diff(&format!("row08 #{i}"), &s, 1, 2);
        n += 1;
    }
    for &v in SPECIALS {
        if f32::from_bits(v).is_nan() {
            continue;
        }
        for &c in SPECIALS {
            let s = [v, v, c];
            assert!(!trace(s[0], s[1], s[2]).arm_if);
            diff("row08 specials", &s, 1, 2);
            n += 1;
        }
    }
    assert!(n >= SAMPLES, "row08: only {n} equal-operand samples");
}

/// ERRORS.md row 9 — guard false because `src[0]` is NaN (unordered compare).
#[test]
fn row09_guard_false_src0_nan() {
    let mut rng = Rng::new(SEED ^ 0x09);
    let mut n = 0;
    for i in 0..2 * SAMPLES {
        let nan = nan_bits(&mut rng);
        let b = rng.pool_f32();
        if f32::from_bits(b).is_nan() {
            continue; // row 11
        }
        let s = [nan, b, rng.pool_f32()];
        assert!(!trace(s[0], s[1], s[2]).arm_if, "row09: expected the else arm");
        diff(&format!("row09 #{i}"), &s, 1, 2);
        n += 1;
    }
    assert!(n >= SAMPLES, "row09: only {n} samples");
}

/// ERRORS.md row 10 — guard false because `src[1]` is NaN (unordered compare).
#[test]
fn row10_guard_false_src1_nan() {
    let mut rng = Rng::new(SEED ^ 0x0a);
    let mut n = 0;
    for i in 0..2 * SAMPLES {
        let a = rng.pool_f32();
        if f32::from_bits(a).is_nan() {
            continue; // row 11
        }
        let nan = nan_bits(&mut rng);
        let s = [a, nan, rng.pool_f32()];
        assert!(!trace(s[0], s[1], s[2]).arm_if, "row10: expected the else arm");
        diff(&format!("row10 #{i}"), &s, 1, 2);
        n += 1;
    }
    assert!(n >= SAMPLES, "row10: only {n} samples");
}

/// ERRORS.md row 11 — guard false because both operands are NaN.
#[test]
fn row11_guard_false_both_nan() {
    let mut rng = Rng::new(SEED ^ 0x0b);
    for i in 0..2 * SAMPLES {
        let s = [nan_bits(&mut rng), nan_bits(&mut rng), rng.pool_f32()];
        assert!(!trace(s[0], s[1], s[2]).arm_if, "row11: expected the else arm");
        diff(&format!("row11 #{i}"), &s, 1, 2);
    }
    // Every NaN x NaN pair from the specials table.
    let nans: Vec<u32> = SPECIALS
        .iter()
        .copied()
        .filter(|&b| f32::from_bits(b).is_nan())
        .collect();
    assert!(nans.len() >= 6);
    for &a in &nans {
        for &b in &nans {
            for &c in SPECIALS {
                assert!(!trace(a, b, c).arm_if);
                diff("row11 specials", &[a, b, c], 1, 2);
            }
        }
    }
}

/// ERRORS.md row 12 — guard false for `-0.0 < +0.0` (signed-zero compare).
#[test]
fn row12_guard_false_signed_zero() {
    let zeros = [0x0000_0000u32, 0x8000_0000u32];
    for &a in &zeros {
        for &b in &zeros {
            for &c in SPECIALS {
                assert!(
                    !trace(a, b, c).arm_if,
                    "row12: ±0 < ±0 must be false, so the else arm is taken"
                );
                diff(&format!("row12 {a:#010x} {b:#010x}"), &[a, b, c], 1, 2);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x0c);
    for i in 0..SAMPLES {
        for &a in &zeros {
            for &b in &zeros {
                diff(
                    &format!("row12 rand #{i}"),
                    &[a, b, rng.pool_f32()],
                    1,
                    2,
                );
            }
        }
    }
}

// ===========================================================================
// Rows 13-18 — the inlined range check `(0 > sqd) ? 0 : sqd`.
// ===========================================================================

/// ERRORS.md row 13 — the clamp **rejects** a negative `sqd`, forcing
/// `sqrtf(+0.0f)`.
#[test]
fn row13_clamp_rejects_negative_sqd() {
    // Constructed witness: x = 0.75 * 2^-75. Then x*x underflows to +0.0 while
    // (x+x)*x rounds up to 2^-149, so sqd = -2^-149 < 0.
    let x = (0.75f64 * 2f64.powi(-75)) as f32;
    let t = trace(x.to_bits(), x.to_bits(), 0.0f32.to_bits());
    assert!(t.sqd < 0.0, "row13 witness must produce sqd < 0, got {:e}", t.sqd);
    assert_eq!(t.clamped.to_bits(), 0, "row13: clamp must yield +0.0f");
    diff1("row13 witness", x.to_bits(), x.to_bits(), 0.0f32.to_bits());

    let hits = diff_matching("row13 clamp-taken", SEED ^ 0x0d, 400_000, 200, |t| {
        t.sqd < 0.0
    });
    println!("row13: {hits} inputs with sqd < 0");
}

/// ERRORS.md row 14 — the clamp does **not** reject a NaN `sqd`
/// (`0 > NaN` is false, unlike `fmaxf`), so `sqrtf` receives the NaN.
#[test]
fn row14_clamp_passes_nan_sqd() {
    let hits = diff_matching("row14 sqd-nan", SEED ^ 0x0e, 200_000, 1000, |t| {
        t.sqd.is_nan()
    });
    println!("row14: {hits} inputs with sqd == NaN");
    // Explicit witnesses: manufactured NaN (inf - inf) and propagated NaN.
    diff1("row14 inf-inf", 0xff80_0000, 0xff80_0000, 0x3f80_0000);
    diff1("row14 propagated", 0x7fc0_1234, 0x3f80_0000, 0x4000_0000);
    for &n in &[0x7fc0_0000u32, 0xffc0_0000, 0x7f80_0001, 0xffbf_ffff] {
        diff1("row14 nan-dxy", 0x3f80_0000, 0x4000_0000, n);
        assert!(trace(0x3f80_0000, 0x4000_0000, n).sqd.is_nan());
    }
}

/// ERRORS.md row 15 — the clamp does not normalize `-0.0`, but `sqd == -0.0` is
/// **unreachable** through the public API: the final addend `(4*dxy)*dxy` always
/// has a positive sign, so `+0.0 + x` can never yield `-0.0`.
#[test]
fn row15_sqd_negative_zero_unreachable() {
    // The addend's sign is structurally non-negative.
    for &c in SPECIALS {
        let t = trace(0x3f80_0000, 0x4000_0000, c);
        assert!(
            t.term4.is_nan() || t.term4.is_sign_positive(),
            "row15: (4*dxy)*dxy must never be negative, got {:e} for dxy {c:#010x}",
            t.term4
        );
    }
    assert_unreachable("row15 sqd==-0.0", SEED ^ 0x0f, 300_000, |t| {
        t.sqd.to_bits() == 0x8000_0000
    });
}

/// ERRORS.md row 16 — clamp boundary `sqd == +0.0` (`0 > 0` is false).
#[test]
fn row16_clamp_boundary_positive_zero() {
    let hits = diff_matching("row16 sqd==+0", SEED ^ 0x10, 300_000, 500, |t| {
        t.sqd.to_bits() == 0
    });
    println!("row16: {hits} inputs with sqd == +0.0");
    // dx2 == dy2 and dxy == 0 gives sqd == +0.0 exactly.
    for &v in SPECIALS {
        if !f32::from_bits(v).is_finite() {
            continue;
        }
        for &z in &[0x0000_0000u32, 0x8000_0000u32] {
            let t = trace(v, v, z);
            if t.sqd.to_bits() == 0 {
                diff1("row16 dx2==dy2", v, v, z);
            }
        }
    }
}

/// ERRORS.md row 17 — one step past the accepted range: `sqd == 0x80000001`
/// (the largest-magnitude-negative smallest subnormal) is clamped.
#[test]
fn row17_clamp_boundary_smallest_negative_subnormal() {
    let x = (0.75f64 * 2f64.powi(-75)) as f32;
    let t = trace(x.to_bits(), x.to_bits(), 0.0f32.to_bits());
    assert_eq!(
        t.sqd.to_bits(),
        0x8000_0001,
        "row17 witness must produce sqd == 0x80000001"
    );
    diff1("row17 witness", x.to_bits(), x.to_bits(), 0.0f32.to_bits());
    let hits = diff_matching("row17 sqd==0x80000001", SEED ^ 0x11, 600_000, 20, |t| {
        t.sqd.to_bits() == 0x8000_0001
    });
    println!("row17: {hits} inputs with sqd == 0x80000001");
}

/// ERRORS.md row 18 — the accepted side of the same boundary:
/// `sqd == 0x00000001` (smallest positive subnormal) reaches `sqrtf`.
#[test]
fn row18_boundary_smallest_positive_subnormal() {
    let hits = diff_matching("row18 sqd==0x00000001", SEED ^ 0x12, 600_000, 20, |t| {
        t.sqd.to_bits() == 0x0000_0001
    });
    println!("row18: {hits} inputs with sqd == 0x00000001");
    // Any positive-subnormal sqd.
    let hits2 = diff_matching("row18 sqd subnormal", SEED ^ 0x13, 300_000, 200, |t| {
        let b = t.sqd.to_bits();
        b != 0 && b < 0x0080_0000
    });
    println!("row18: {hits2} inputs with subnormal positive sqd");
}

// ===========================================================================
// Rows 19-26 — IEEE-754 invalid/overflow "errors" and their exact bit patterns.
// ===========================================================================

/// ERRORS.md row 19 — `dy2*dy2` overflows to `+inf`.
#[test]
fn row19_square_overflow_to_inf() {
    let hits = diff_matching("row19 dy2^2 overflow", SEED ^ 0x14, 200_000, 500, |t| {
        t.dy2.is_finite() && t.dy2_sq.is_infinite()
    });
    println!("row19: {hits} inputs where dy2*dy2 overflows");
    // FLT_MAX and just above the sqrt(FLT_MAX) threshold.
    for &v in &[0x7f7f_ffffu32, 0xff7f_ffff, 0x5f00_0000, 0xdf00_0000] {
        diff1("row19 witness-a", v, 0x3f80_0000, 0x0000_0000);
        diff1("row19 witness-b", 0x3f80_0000, v, 0x0000_0000);
    }
}

/// ERRORS.md row 20 — `inf - inf` inside `sqd` yields the x86 indefinite QNaN
/// `0xffc00000` (sign bit **set**), not `0x7fc00000`.
#[test]
fn row20_inf_minus_inf_indefinite() {
    // Witness: dy2 = dx2 = -inf  =>  dy2^2 = +inf and (dx2+dx2)*dy2 = +inf.
    let t = trace(0xff80_0000, 0xff80_0000, 0x3f80_0000);
    assert!(t.dy2_sq.is_infinite() && t.two_dx2_dy2.is_infinite());
    assert_eq!(t.dy2_sq.is_sign_positive(), t.two_dx2_dy2.is_sign_positive());
    assert!(t.after_sub.is_nan(), "row20: inf - inf must be NaN");
    diff1("row20 witness", 0xff80_0000, 0xff80_0000, 0x3f80_0000);

    let hits = diff_matching("row20 inf-inf", SEED ^ 0x15, 200_000, 500, |t| {
        t.dy2_sq.is_infinite()
            && t.two_dx2_dy2.is_infinite()
            && t.dy2_sq.is_sign_positive() == t.two_dx2_dy2.is_sign_positive()
    });
    println!("row20: {hits} inputs with inf - inf in sqd");
}

/// ERRORS.md row 21 — `0.0f * inf`.
///
/// * Reachable inside `2.0f*dx2*dy2` (`dx2 == 0`, `dy2 == ±inf`, or vice versa).
/// * **Unreachable** inside `4.0f*dxy*dxy`: the two factors of `(4*dxy)*dxy`
///   cannot be `0` and `inf` simultaneously, since `|4*dxy| >= |dxy|`.
#[test]
fn row21_zero_times_inf() {
    // Reachable site: (dx2+dx2) * dy2.
    let t = trace(0x0000_0000, 0x7f80_0000, 0x0000_0000);
    assert!(!t.dx2.is_nan() && !t.dy2.is_nan() && t.two_dx2_dy2.is_nan());
    diff1("row21a witness", 0x0000_0000, 0x7f80_0000, 0x0000_0000);
    for &z in &[0x0000_0000u32, 0x8000_0000u32] {
        for &inf in &[0x7f80_0000u32, 0xff80_0000u32] {
            for &c in SPECIALS {
                diff1("row21a zero-inf", z, inf, c);
                diff1("row21a inf-zero", inf, z, c);
            }
        }
    }
    let hits = diff_matching("row21a 0*inf", SEED ^ 0x16, 200_000, 100, |t| {
        !t.dx2.is_nan() && !t.dy2.is_nan() && t.two_dx2_dy2.is_nan()
    });
    println!("row21a: {hits} inputs with 0*inf in 2*dx2*dy2");

    // Unreachable site: (4*dxy)*dxy.
    assert_unreachable("row21b 0*inf in 4*dxy*dxy", SEED ^ 0x17, 200_000, |t| {
        !t.dxy.is_nan() && t.term4.is_nan()
    });
}

/// ERRORS.md row 22 — `inf + (-inf)` in `dy2 + dx2`.
#[test]
fn row22_inf_plus_neg_inf_in_sum() {
    let t = trace(0xff80_0000, 0x7f80_0000, 0x0000_0000);
    assert!(t.dy2.is_infinite() && t.dx2.is_infinite() && (t.dy2 + t.dx2).is_nan());
    diff1("row22 witness", 0xff80_0000, 0x7f80_0000, 0x0000_0000);
    for &c in SPECIALS {
        diff1("row22 -inf,+inf", 0xff80_0000, 0x7f80_0000, c);
        diff1("row22 +inf,-inf", 0x7f80_0000, 0xff80_0000, c);
    }
    let hits = diff_matching("row22 inf+-inf", SEED ^ 0x18, 200_000, 50, |t| {
        t.dy2.is_infinite() && t.dx2.is_infinite() && (t.dy2 + t.dx2).is_nan()
    });
    println!("row22: {hits} inputs with inf + -inf in dy2 + dx2");
}

/// ERRORS.md row 23 — `sqrtf` can never see a negative argument, so glibc's
/// `__math_invalidf` domain-error path is unreachable from `tfm`.
#[test]
fn row23_sqrtf_never_sees_negative() {
    assert_unreachable("row23 sqrtf(negative)", SEED ^ 0x19, 400_000, |t| {
        // strictly negative, i.e. excluding -0.0 which is not a domain error
        t.clamped < 0.0
    });
    // -0.0 is likewise never handed to sqrtf (row 15).
    assert_unreachable("row23 sqrtf(-0.0)", SEED ^ 0x1a, 200_000, |t| {
        t.clamped.to_bits() == 0x8000_0000
    });
}

/// ERRORS.md row 24 — signaling NaN operands get **quieted** (`| 0x00400000`)
/// with sign and remaining payload preserved.
#[test]
fn row24_signaling_nan_quieted() {
    let snans: Vec<u32> = (0..48)
        .map(|k| {
            let sign = (k & 1) << 31;
            let payload = 1 + (k as u32) * 0x0002_1234;
            sign | 0x7f80_0000 | (payload & 0x003f_ffff).max(1)
        })
        .filter(|b| b & 0x0040_0000 == 0 && b & 0x003f_ffff != 0)
        .collect();
    assert!(snans.len() >= 20, "need signaling NaNs, got {}", snans.len());
    for &n in &snans {
        assert!(f32::from_bits(n).is_nan() && n & 0x0040_0000 == 0);
        for &other in SPECIALS {
            diff1("row24 snan in slot 0", n, other, other);
            diff1("row24 snan in slot 1", other, n, other);
            diff1("row24 snan in slot 2", other, other, n);
        }
    }
    let mut rng = Rng::new(SEED ^ 0x1b);
    for i in 0..4 * SAMPLES {
        let n = snans[rng.below(snans.len())];
        let slot = rng.below(3);
        let mut s = [rng.pool_f32(), rng.pool_f32(), rng.pool_f32()];
        s[slot] = n;
        diff(&format!("row24 rand #{i}"), &s, 1, 2);
    }
}

/// ERRORS.md row 25 — with two NaN operands the SSE **destination** operand's
/// payload wins. Exercised by feeding distinct payloads into every slot.
#[test]
fn row25_two_nan_operands_destination_wins() {
    let nans = [
        0x7fc0_0000u32,
        0xffc0_0000,
        0x7fc0_0001,
        0xffff_ffff,
        0x7f80_0001,
        0xffbf_ffff,
        0x7fab_cdef,
        0xff87_6543,
    ];
    for &a in &nans {
        for &b in &nans {
            for &c in &nans {
                diff1("row25 all-nan", a, b, c);
            }
            for &c in SPECIALS {
                diff1("row25 two-nan", a, b, c);
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0x1c);
    for i in 0..8 * SAMPLES {
        let s = [nan_bits(&mut rng), nan_bits(&mut rng), nan_bits(&mut rng)];
        diff(&format!("row25 rand #{i}"), &s, 1, 2);
    }
}

/// ERRORS.md row 26 — `dx2 - lambda` with both operands infinite and of the
/// same sign is **unreachable**: whenever `dx2` is infinite, `sqd` is either
/// NaN or `+inf` in a way that forces `lambda` to be NaN, and `lambda` can
/// never be `-inf`.
#[test]
fn row26_inf_minus_inf_in_output_unreachable() {
    assert_unreachable("row26 dx2-lambda inf-inf", SEED ^ 0x1d, 300_000, |t| {
        t.dx2.is_infinite()
            && t.lambda.is_infinite()
            && t.dx2.is_sign_positive() == t.lambda.is_sign_positive()
    });
    // Corollary that makes it unreachable: lambda is never -inf.
    assert_unreachable("row26 lambda==-inf", SEED ^ 0x1e, 300_000, |t| {
        t.lambda == f32::NEG_INFINITY
    });
}

// ===========================================================================
// Row 27 — aliasing (no `restrict`, no overlap check).
// ===========================================================================

/// ERRORS.md row 27 — `dest` aliases `src`.
#[test]
fn row27_aliasing_dest_and_src() {
    let mut rng = Rng::new(SEED ^ 0x1f);
    for i in 0..SAMPLES {
        let n = 1 + rng.below(8);
        let buf: Vec<u32> = (0..3 * n + 8).map(|_| rng.pool_f32()).collect();
        // exact aliasing
        diff_alias(&format!("row27 exact #{i}"), &buf, n as i32, 0, 0);
        // dest ahead of src (destructive: writes clobber unread inputs)
        diff_alias(&format!("row27 dest-ahead #{i}"), &buf, n as i32, 1, 0);
        diff_alias(&format!("row27 dest-ahead2 #{i}"), &buf, n as i32, 2, 0);
        // dest behind src (benign)
        diff_alias(&format!("row27 dest-behind #{i}"), &buf, n as i32, 0, 1);
        diff_alias(&format!("row27 dest-behind2 #{i}"), &buf, n as i32, 0, 2);
    }
}

// ===========================================================================
// Row 28 — arbitrary 32-bit patterns for `count` (the only scalar parameter;
// the API declares no enum, so this is the full "out-of-range scalar" surface).
// ===========================================================================

/// ERRORS.md row 28 — every extreme `int` pattern for `count`. Only
/// non-positive patterns have a defined result (positive huge counts would read
/// out of bounds, which the C does not check).
#[test]
fn row28_extreme_count_patterns() {
    let src = vec![
        0x3f80_0000u32,
        0x4000_0000,
        0x4040_0000,
        0xff80_0000,
        0x7fc0_1234,
        0x0000_0001,
    ];
    let counts: Vec<i32> = vec![
        0,
        -1,
        1,
        2,
        i32::MIN,
        i32::MIN + 1,
        -i32::MAX,
        -0x4000_0000,
        -0x0000_8000,
        -0x0000_0002,
    ];
    for &count in &counts {
        let out = if count > 0 { 2 * count as usize } else { 2 };
        if count > 0 && 3 * count as usize > src.len() {
            continue;
        }
        diff(&format!("row28 count={count}"), &src, count, out);
    }
    // A dense sweep of non-positive counts.
    for count in -512i32..=0 {
        diff(&format!("row28 sweep count={count}"), &src, count, 2);
    }
    // ...and of small positive counts against a matching buffer.
    let mut rng = Rng::new(SEED ^ 0x20);
    for count in 1i32..=64 {
        let s: Vec<u32> = (0..3 * count as usize).map(|_| rng.pool_f32()).collect();
        diff(&format!("row28 positive count={count}"), &s, count, 2 * count as usize);
    }
}

// ---------------------------------------------------------------------------

/// A random NaN: random sign, random payload, random quiet bit.
fn nan_bits(rng: &mut Rng) -> u32 {
    let sign = (rng.next_u64() & 1) as u32;
    let payload = (rng.next_u32() & 0x007f_ffff).max(1);
    (sign << 31) | 0x7f80_0000 | payload
}
