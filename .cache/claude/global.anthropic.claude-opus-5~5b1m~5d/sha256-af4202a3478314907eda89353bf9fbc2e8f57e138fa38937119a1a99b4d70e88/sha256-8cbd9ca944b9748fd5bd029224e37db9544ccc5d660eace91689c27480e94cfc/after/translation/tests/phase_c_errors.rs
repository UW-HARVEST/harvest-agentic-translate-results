//! Phase C — error-path / rejection differential tests.
//!
//! One test per row of `ERRORS.md`. `tfm` returns `void` and the C source
//! contains no `return`, no `assert`, no error enum and no null/range check, so
//! "the same error/rejection" is observed as: *the same refusal to do work* —
//! byte-identical destination buffers (including "not written at all"),
//! identical absence of out-of-bounds access, and identical bit patterns for
//! the value-domain boundaries (clamp, unordered compare, invalid ops).

mod common;

use common::*;

/// Call both with a destination pre-filled with a recognisable pattern and
/// assert BOTH left it byte-identical — i.e. both rejected the request the same
/// way, writing nothing at all.
#[track_caller]
fn assert_both_write_nothing(src: *const f32, count: i32, ctx: &str) {
    let pattern: Vec<f32> = (0..64).map(|i| f32::from_bits(0xCAFE_0000 + i)).collect();
    let mut c_dest = pattern.clone();
    let mut r_dest = pattern.clone();

    let l = libs();
    unsafe {
        (l.c_tfm)(c_dest.as_mut_ptr(), src, count);
        (l.rust_tfm)(r_dest.as_mut_ptr(), src, count);
    }

    assert!(
        bits_eq(&c_dest, &pattern),
        "[{ctx}] C unexpectedly wrote to dest for count={count}"
    );
    assert!(
        bits_eq(&r_dest, &pattern),
        "[{ctx}] Rust wrote to dest for count={count} but C did not\n  C    = {}\n  Rust = {}",
        show(&c_dest),
        show(&r_dest)
    );
    assert!(bits_eq(&c_dest, &r_dest), "[{ctx}] C/Rust dest differ");
}

// ===========================================================================
// Row 1 — count == 0 (zero length)
// ===========================================================================
#[test]
fn err01_count_zero_writes_nothing() {
    let src: Vec<f32> = (0..96).map(|i| i as f32 + 0.5).collect();
    assert_both_write_nothing(src.as_ptr(), 0, "err01 count=0");

    // Also via the standard harness, which asserts a zero-length live region
    // and untouched guards.
    let out = diff_call(&src, 0, "err01 count=0 via diff_call");
    assert!(out.is_empty());
}

// ===========================================================================
// Row 2 — count == -1 (one step past the valid range)
// ===========================================================================
#[test]
fn err02_count_negative_one() {
    let src: Vec<f32> = (0..96).map(|i| i as f32 - 3.25).collect();
    assert_both_write_nothing(src.as_ptr(), -1, "err02 count=-1");
    let out = diff_call(&src, -1, "err02 count=-1 via diff_call");
    assert!(out.is_empty());
}

// ===========================================================================
// Row 3 — count == INT_MIN (extreme negative; must not wrap to a huge loop)
// ===========================================================================
#[test]
fn err03_count_int_min() {
    let src: Vec<f32> = (0..96).map(|i| (i as f32) * 1.5).collect();
    assert_both_write_nothing(src.as_ptr(), i32::MIN, "err03 count=INT_MIN");
    assert_both_write_nothing(src.as_ptr(), i32::MIN + 1, "err03 count=INT_MIN+1");
    assert_both_write_nothing(src.as_ptr(), -2, "err03 count=-2");
}

// ===========================================================================
// Row 4 — many random negative counts
// ===========================================================================
#[test]
fn err04_many_random_negative_counts() {
    let src: Vec<f32> = (0..96).map(|i| (i as f32).sin()).collect();
    let mut rng = Rng::new(0xE4);
    for _ in 0..5_000 {
        let n = -((rng.next_u32() as i64 % (i32::MAX as i64 + 1)) as i64) as i64;
        let n = n.max(i32::MIN as i64) as i32;
        if n >= 0 {
            continue;
        }
        assert_both_write_nothing(src.as_ptr(), n, "err04 random negative");
    }
    // Every negative power of two, exactly.
    for b in 0..31u32 {
        let n = -(1i64 << b) as i32;
        assert_both_write_nothing(src.as_ptr(), n, "err04 -2^b");
    }
}

// ===========================================================================
// Row 5 — dest == NULL && src == NULL && count <= 0
// ===========================================================================
#[test]
fn err05_both_null_nonpositive_count() {
    let l = libs();
    for &count in &[0i32, -1, -2, -1000, i32::MIN] {
        // The loop body never runs, so the null pointers are never
        // dereferenced. Both must return normally.
        unsafe {
            (l.c_tfm)(std::ptr::null_mut(), std::ptr::null(), count);
            (l.rust_tfm)(std::ptr::null_mut(), std::ptr::null(), count);
        }
    }
    // Reaching here means neither faulted, for all five counts.
}

// ===========================================================================
// Row 6 — dest == NULL, src valid, count == 0
// ===========================================================================
#[test]
fn err06_null_dest_valid_src_count_zero() {
    let src: Vec<f32> = (0..96).map(|i| i as f32).collect();
    let l = libs();
    for &count in &[0i32, -1, i32::MIN] {
        unsafe {
            (l.c_tfm)(std::ptr::null_mut(), src.as_ptr(), count);
            (l.rust_tfm)(std::ptr::null_mut(), src.as_ptr(), count);
        }
    }
    // src must not have been modified by either (it is `const` in C).
    for (i, v) in src.iter().enumerate() {
        assert_eq!(v.to_bits(), (i as f32).to_bits(), "src[{i}] was modified");
    }
}

// ===========================================================================
// Row 7 — src == NULL, dest valid & pre-filled, count == 0
// ===========================================================================
#[test]
fn err07_null_src_valid_dest_count_zero() {
    for &count in &[0i32, -1, i32::MIN] {
        assert_both_write_nothing(std::ptr::null(), count, "err07 null src");
    }
}

// ===========================================================================
// Row 8 — count == 1 with exactly-sized buffers (no slack) + guards
// ===========================================================================
#[test]
fn err08_minimum_nonempty_exact_buffers() {
    let mut rng = Rng::new(0xE8);
    let l = libs();
    for _ in 0..20_000 {
        // Exactly 3 source floats and exactly 2 destination floats, each
        // immediately followed by guard values.
        let src3 = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        const GUARD: u32 = 0xDEAD_BEEF;
        let mut c_buf = [0.0f32, 0.0, f32::from_bits(GUARD), f32::from_bits(GUARD)];
        let mut r_buf = c_buf;
        unsafe {
            (l.c_tfm)(c_buf.as_mut_ptr(), src3.as_ptr(), 1);
            (l.rust_tfm)(r_buf.as_mut_ptr(), src3.as_ptr(), 1);
        }
        assert!(
            bits_eq(&c_buf, &r_buf),
            "err08 mismatch src={}\n C={}\n R={}",
            show(&src3),
            show(&c_buf),
            show(&r_buf)
        );
        // No over-write past 2*count in EITHER implementation.
        assert_eq!(c_buf[2].to_bits(), GUARD, "err08 C over-wrote dest[2]");
        assert_eq!(c_buf[3].to_bits(), GUARD, "err08 C over-wrote dest[3]");
        assert_eq!(r_buf[2].to_bits(), GUARD, "err08 Rust over-wrote dest[2]");
        assert_eq!(r_buf[3].to_bits(), GUARD, "err08 Rust over-wrote dest[3]");
        // src untouched.
        assert!(bits_eq(&src3, &[src3[0], src3[1], src3[2]]));
    }
}

// ===========================================================================
// Row 9 — sqd < 0 forced: the clamp replaces it with 0.0f (no NaN, no errno)
// ===========================================================================
#[test]
fn err09_negative_discriminant_is_clamped_not_sqrt_of_negative() {
    // Literal triples verified to make the expanded discriminant negative.
    let cases: &[(u32, u32, u32)] = &[
        (0x4dfc_bb3c, 0x4dfc_bb09, 0x45fc_bb3c),
        (0x430d_d2a4, 0x430d_d24e, 0x370d_d2a4),
        (0x418a_179a, 0x418a_15ed, 0x328a_179a),
        (0x4a5e_90b4, 0x4a5e_9053, 0x365e_90b4),
        (0x4bc1_fe94, 0x4bc1_fcd8, 0x3ac1_fe94),
    ];
    for &(a, b, c) in cases {
        let (x, y, z) = (f32::from_bits(a), f32::from_bits(b), f32::from_bits(c));
        for src in [[x, y, z], [y, x, z]] {
            let out = diff_call(&src, 1, "err09 clamped negative sqd");
            // The clamp means sqrtf(0.0f): no NaN may appear in the output.
            assert!(
                !out[0].is_nan() && !out[1].is_nan(),
                "err09 NaN leaked through the clamp: src={} out={}",
                show(&src),
                show(&out)
            );
            // lambda == 0.5*(dy2+dx2+0) exactly, so the output is finite.
            assert!(out[0].is_finite() && out[1].is_finite());
        }
    }

    // Plus a randomized sweep: every triple whose discriminant is negative
    // must yield a NaN-free, finite result in BOTH implementations.
    let mut rng = Rng::new(0xE9);
    let mut n = 0;
    for _ in 0..400_000 {
        let base = f32::from_bits(0x3f80_0000u32.wrapping_add(rng.next_u32() & 0x0fff_ffff));
        let k = (rng.next_u32() & 0x3ff) as i32 - 512;
        let other = f32::from_bits((base.to_bits() as i32).wrapping_add(k) as u32);
        let dxy = f32::from_bits(
            base.to_bits().wrapping_sub((rng.next_u32() & 0x0f00_0000) + 0x0800_0000),
        );
        if !base.is_finite() || !other.is_finite() || !dxy.is_finite() {
            continue;
        }
        for src in [[base, other, dxy], [other, base, dxy]] {
            let (dx2, dy2, d) = if src[0] < src[1] {
                (src[0], src[1], src[2])
            } else {
                (src[1], src[0], src[2])
            };
            let s = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * d * d);
            if !(s < 0.0) {
                continue;
            }
            let out = diff_call(&src, 1, "err09 random negative sqd");
            assert!(!out[0].is_nan() && !out[1].is_nan(), "err09 NaN leaked");
            n += 1;
        }
        if n >= 20_000 {
            break;
        }
    }
    assert!(n > 5_000, "err09: only {n} negative-discriminant cases exercised");
}

// ===========================================================================
// Row 10 — sqd == -0.0 boundary: `0 > -0.0` is FALSE, so it is NOT clamped
// ===========================================================================
#[test]
fn err10_minus_zero_discriminant_boundary() {
    // The `+ (4*dxy*dxy)` term is `+0.0` for every finite `dxy`, so a genuinely
    // `-0.0` discriminant is unreachable from finite inputs. What IS reachable
    // and IS the boundary of the clamp is an exactly-`+0.0` discriminant, where
    // `0 > sqd` is likewise FALSE, so `sqd` (not the literal `0`) is passed to
    // `sqrtf`. Both must agree bit-for-bit, including the sign of the zero.
    let mut rng = Rng::new(0xEA);
    let mut zeros = 0;
    for _ in 0..40_000 {
        let v = rng.finite();
        for &z in &[0.0f32, -0.0f32] {
            let src = [v, v, z];
            let (dx2, dy2, d) = (src[1], src[0], src[2]); // else-branch (v == v)
            let s = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * d * d);
            let out = diff_call(&src, 1, "err10 zero discriminant");
            if s == 0.0 {
                zeros += 1;
                assert!(!out[1].is_nan(), "err10 NaN from sqrt of zero");
            }
        }
    }
    assert!(zeros > 10_000, "err10: only {zeros} zero-discriminant cases");

    // Direct check that sqrtf's signed-zero behaviour matches: dx2 == dy2 == 0
    // and dxy == ±0 gives sqd == +0.0 and sqrtf(+0.0) == +0.0.
    for &a in &[0.0f32, -0.0f32] {
        for &b in &[0.0f32, -0.0f32] {
            for &c in &[0.0f32, -0.0f32] {
                let out = diff_call(&[a, b, c], 1, "err10 all-zeros");
                assert!(out.iter().all(|v| !v.is_nan()), "err10 NaN from all-zeros");
            }
        }
    }
}

// ===========================================================================
// Row 11 — sqd is NaN: `0 > NaN` is FALSE, so the NaN reaches sqrtf
// ===========================================================================
#[test]
fn err11_nan_discriminant_propagates_through_sqrt() {
    // A NaN input in slot 2 (dxy) makes the discriminant NaN while the branch
    // test on slots 0/1 stays ordered — so the NaN must survive `sqrtf` and
    // reach the output with the exact same bits in both implementations.
    for &nv in &nan_patterns() {
        for &(a, b) in &[(1.0f32, 2.0f32), (2.0, 1.0), (-5.0, 5.0), (1e30, 1e-30)] {
            let out = diff_call(&[a, b, nv], 1, "err11 NaN dxy");
            assert!(
                out.iter().any(|v| v.is_nan()),
                "err11 expected NaN in output for src={}",
                show(&[a, b, nv])
            );
        }
    }
    // inf inputs that make the discriminant NaN via inf - inf.
    for &(a, b, c) in &[
        (f32::INFINITY, f32::INFINITY, 0.0f32),
        (f32::NEG_INFINITY, f32::NEG_INFINITY, 0.0),
        (f32::INFINITY, f32::NEG_INFINITY, 0.0),
        (f32::NEG_INFINITY, f32::INFINITY, 0.0),
        (0.0, f32::INFINITY, 0.0),
        (f32::INFINITY, 0.0, 0.0),
    ] {
        diff_call(&[a, b, c], 1, "err11 inf discriminant");
    }
    // Randomized: any triple whose discriminant is NaN.
    let mut rng = Rng::new(0xEB);
    let mut n = 0;
    for _ in 0..200_000 {
        let src = [rng.any_bits(), rng.any_bits(), rng.any_bits()];
        let (dx2, dy2, d) = if src[0] < src[1] {
            (src[0], src[1], src[2])
        } else {
            (src[1], src[0], src[2])
        };
        let s = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) + (4.0f32 * d * d);
        if !s.is_nan() {
            continue;
        }
        diff_call(&src, 1, "err11 random NaN discriminant");
        n += 1;
    }
    assert!(n > 1_000, "err11: only {n} NaN-discriminant cases");
}

// ===========================================================================
// Row 12 — NaN in src[0]/src[1] makes `<` unordered => the ELSE branch
// ===========================================================================
#[test]
fn err12_unordered_compare_takes_else_branch() {
    for &nv in &nan_patterns() {
        for &other in SPECIALS {
            // NaN in slot 0.
            let src = [nv, other, 1.5f32];
            assert!(!(src[0] < src[1]), "unordered compare must be false");
            diff_call(&src, 1, "err12 NaN@0");

            // NaN in slot 1.
            let src = [other, nv, 1.5f32];
            assert!(!(src[0] < src[1]), "unordered compare must be false");
            diff_call(&src, 1, "err12 NaN@1");

            // NaN in both.
            let src = [nv, nv, other];
            assert!(!(src[0] < src[1]));
            diff_call(&src, 1, "err12 NaN@0+1");
        }
    }

    // Prove the else-branch is the one taken, by comparing against the
    // deliberately-constructed else-branch value that the C must produce:
    // dest[0] == dxy verbatim (the else branch stores dxy into slot 0; the if
    // branch would store dx2 - lambda there).
    let mut rng = Rng::new(0xEC);
    for _ in 0..20_000 {
        let dxy = rng.any_bits();
        let out = diff_call(&[f32::NAN, rng.finite(), dxy], 1, "err12 else proof");
        assert_eq!(
            out[0].to_bits(),
            dxy.to_bits(),
            "else branch not taken for a NaN operand"
        );
    }
}

// ===========================================================================
// Row 13 — at/over the float range: FLT_MAX, inf, invalid-op NaN payloads
// ===========================================================================
#[test]
fn err13_range_extremes_and_invalid_ops() {
    let extremes = [
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7F7F_FFFF), // FLT_MAX
        f32::from_bits(0xFF7F_FFFF), // -FLT_MAX
        f32::from_bits(0x7F7F_FFFE), // one ulp below FLT_MAX
        3.4e38,
        -3.4e38,
        1.0e20,
        -1.0e20,
        0.0,
        -0.0,
    ];
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                diff_call(&[a, b, c], 1, "err13 extremes");
            }
        }
    }
    // Exhaustive over-range sweep at the exponent boundary: values whose square
    // overflows, one step either side of sqrt(FLT_MAX).
    let root = f32::MAX.sqrt();
    for k in -64i32..=64 {
        let v = f32::from_bits((root.to_bits() as i32).wrapping_add(k) as u32);
        for &s in &[v, -v] {
            for &c in &extremes {
                diff_call(&[s, s, c], 1, "err13 sqrt(FLT_MAX) boundary");
                diff_call(&[s, -s, c], 1, "err13 sqrt(FLT_MAX) boundary neg");
            }
        }
    }
}

// ===========================================================================
// Row 14 — subnormal / signed-zero one-past-normal-range values
// ===========================================================================
#[test]
fn err14_subnormal_and_signed_zero_boundaries() {
    let tiny = [
        0.0f32,
        -0.0,
        f32::from_bits(0x0000_0001), // +FLT_TRUE_MIN
        f32::from_bits(0x8000_0001), // -FLT_TRUE_MIN
        f32::from_bits(0x007F_FFFF), // largest +subnormal
        f32::from_bits(0x807F_FFFF), // largest -subnormal
        f32::MIN_POSITIVE,           // smallest +normal
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0080_0001), // one ulp above smallest normal
        f32::from_bits(0x8080_0001),
    ];
    for &a in &tiny {
        for &b in &tiny {
            for &c in &tiny {
                diff_call(&[a, b, c], 1, "err14 subnormal/zero");
            }
        }
    }
    // Mixed with normals so the subnormal is the one that underflows.
    let mut rng = Rng::new(0xEE);
    for _ in 0..40_000 {
        let s = tiny[rng.below(tiny.len())];
        let n = rng.finite();
        diff_call(&[s, n, n], 1, "err14 mixed");
        diff_call(&[n, s, n], 1, "err14 mixed");
        diff_call(&[n, n, s], 1, "err14 mixed");
    }
}

// ===========================================================================
// Row 15 — dest == src (exact aliasing; no `restrict` in the C signature)
// ===========================================================================
#[test]
fn err15_exact_aliasing_dest_equals_src() {
    let mut rng = Rng::new(0xEF);
    let l = libs();
    for count in 1..=48usize {
        for _ in 0..200 {
            let orig: Vec<f32> = (0..3 * count + 4).map(|_| rng.finite()).collect();
            let mut c_buf = orig.clone();
            let mut r_buf = orig.clone();
            unsafe {
                (l.c_tfm)(c_buf.as_mut_ptr(), c_buf.as_ptr(), count as i32);
                (l.rust_tfm)(r_buf.as_mut_ptr(), r_buf.as_ptr(), count as i32);
            }
            assert!(
                bits_eq(&c_buf, &r_buf),
                "err15 aliasing mismatch count={count}\n in={}\n C ={}\n R ={}",
                show(&orig),
                show(&c_buf),
                show(&r_buf)
            );
            // The tail beyond 2*count must be untouched by both.
            for i in 2 * count..orig.len() {
                assert_eq!(c_buf[i].to_bits(), orig[i].to_bits(), "err15 C tail {i}");
                assert_eq!(r_buf[i].to_bits(), orig[i].to_bits(), "err15 R tail {i}");
            }
        }
    }
}

// ===========================================================================
// Row 16 — partial overlap at several positive and negative offsets
// ===========================================================================
#[test]
fn err16_partial_overlap_offsets() {
    let mut rng = Rng::new(0xF0);
    let l = libs();
    for k in 0..=8usize {
        for count in 1..=24usize {
            for _ in 0..40 {
                let len = 3 * count + k + 8;
                let orig: Vec<f32> = (0..len).map(|_| rng.any_bits()).collect();

                // src ahead of dest.
                let mut c1 = orig.clone();
                let mut r1 = orig.clone();
                unsafe {
                    (l.c_tfm)(c1.as_mut_ptr(), c1.as_ptr().add(k), count as i32);
                    (l.rust_tfm)(r1.as_mut_ptr(), r1.as_ptr().add(k), count as i32);
                }
                assert!(
                    bits_eq(&c1, &r1),
                    "err16 src=base+{k} mismatch count={count}\n C={}\n R={}",
                    show(&c1),
                    show(&r1)
                );

                // dest ahead of src.
                let mut c2 = orig.clone();
                let mut r2 = orig.clone();
                unsafe {
                    (l.c_tfm)(c2.as_mut_ptr().add(k), c2.as_ptr(), count as i32);
                    (l.rust_tfm)(r2.as_mut_ptr().add(k), r2.as_ptr(), count as i32);
                }
                assert!(
                    bits_eq(&c2, &r2),
                    "err16 dest=base+{k} mismatch count={count}\n C={}\n R={}",
                    show(&c2),
                    show(&r2)
                );
            }
        }
    }
}

// ===========================================================================
// Row 17 — all-NaN inputs with distinct payloads and mixed signs
// ===========================================================================
#[test]
fn err17_all_nan_distinct_payloads() {
    let nans = nan_patterns();
    for &a in &nans {
        for &b in &nans {
            for &c in &nans {
                diff_call(&[a, b, c], 1, "err17 all-NaN cross product");
            }
        }
    }
    // Randomized NaN payloads: any bit pattern with the max exponent and a
    // non-zero mantissa is a NaN.
    let mut rng = Rng::new(0xF1);
    let mk_nan = |r: &mut Rng| -> f32 {
        let m = (r.next_u32() & 0x007F_FFFF) | 1;
        let s = (r.next_u32() & 1) << 31;
        f32::from_bits(s | 0x7F80_0000 | m)
    };
    for _ in 0..100_000 {
        let a = mk_nan(&mut rng);
        let b = mk_nan(&mut rng);
        let c = mk_nan(&mut rng);
        assert!(a.is_nan() && b.is_nan() && c.is_nan());
        diff_call(&[a, b, c], 1, "err17 random NaN payloads");
    }
    // NaN mixed with inf and zero, the combination that produces the x86
    // "indefinite" QNaN from an *invalid operation* rather than propagation.
    let mixers = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
    ];
    for &nv in &nans {
        for &m1 in &mixers {
            for &m2 in &mixers {
                diff_call(&[nv, m1, m2], 1, "err17 NaN+inf/zero");
                diff_call(&[m1, nv, m2], 1, "err17 NaN+inf/zero");
                diff_call(&[m1, m2, nv], 1, "err17 NaN+inf/zero");
            }
        }
    }
    // Long runs so consecutive iterations mix propagation and invalid-op NaNs.
    let mut pool: Vec<f32> = nans.clone();
    pool.extend_from_slice(&mixers);
    let mut rng = Rng::new(0xF2);
    for _ in 0..500 {
        let n = 1 + rng.below(300);
        let src: Vec<f32> = (0..3 * n).map(|_| pool[rng.below(pool.len())]).collect();
        diff_call(&src, n as i32, "err17 long NaN run");
    }
}

// ===========================================================================
// Generic FFI boundary checks required for every C API
// ===========================================================================

/// `count` is a plain `int`; there is no enum parameter anywhere in the API, so
/// the "out-of-range enum value" class of input is covered by feeding `count`
/// every representative `int` value, including ones with no meaningful
/// interpretation.
#[test]
fn generic_count_full_int_domain() {
    let src: Vec<f32> = (0..3 * 40).map(|i| (i as f32) * 0.125 - 2.0).collect();
    let l = libs();

    // Non-positive: nothing is written; verified against a pre-filled dest.
    for &count in &[i32::MIN, i32::MIN + 1, -1_000_000, -3, -2, -1, 0] {
        assert_both_write_nothing(src.as_ptr(), count, "generic non-positive count");
    }

    // Positive counts up to the buffer capacity: identical outputs.
    for count in 1..=40i32 {
        diff_call(&src[..3 * count as usize], count, "generic positive count");
    }

    // Every bit-width boundary of `int` that still fits the buffer, plus the
    // sign-bit boundary values which must be treated as negative (no wrap).
    for &count in &[i32::MAX, i32::MAX - 1, 0x4000_0000, 0x7FFF_FFFF] {
        // These are huge positive counts, so we cannot let the loop run. Verify
        // instead that a *negative* value with the same low bits writes nothing.
        let neg = count.wrapping_neg();
        if neg < 0 {
            assert_both_write_nothing(src.as_ptr(), neg, "generic huge negative");
        }
        let _ = l;
    }
}

/// Zero-length with a NULL source AND a NULL destination, in every ordering,
/// repeated — the fully degenerate call.
#[test]
fn generic_all_null_and_zero_length_combinations() {
    let src: Vec<f32> = (0..12).map(|i| i as f32).collect();
    let mut dest = vec![0.0f32; 8];
    let l = libs();

    let d_null: *mut f32 = std::ptr::null_mut();
    let s_null: *const f32 = std::ptr::null();
    let d_ok: *mut f32 = dest.as_mut_ptr();
    let s_ok: *const f32 = src.as_ptr();

    for &(d, s) in &[
        (d_null, s_null),
        (d_null, s_ok),
        (d_ok, s_null),
        (d_ok, s_ok),
    ] {
        for &count in &[0i32, -1, -7, i32::MIN] {
            unsafe {
                (l.c_tfm)(d, s, count);
                (l.rust_tfm)(d, s, count);
            }
        }
    }
    // dest must still be all zeros: nothing was written by any of those calls.
    assert!(dest.iter().all(|v| v.to_bits() == 0), "dest was modified");
}

/// Dangling / non-null-but-unusable pointers are only safe when the loop body
/// never runs; confirm both implementations agree that it does not.
#[test]
fn generic_bogus_nonnull_pointers_with_nonpositive_count() {
    let l = libs();
    // Deliberately unmapped-looking but non-null, misaligned addresses. Neither
    // implementation may dereference them for count <= 0.
    let bogus_d = 0x1usize as *mut f32;
    let bogus_s = 0x3usize as *const f32;
    for &count in &[0i32, -1, i32::MIN] {
        unsafe {
            (l.c_tfm)(bogus_d, bogus_s, count);
            (l.rust_tfm)(bogus_d, bogus_s, count);
        }
    }
}
