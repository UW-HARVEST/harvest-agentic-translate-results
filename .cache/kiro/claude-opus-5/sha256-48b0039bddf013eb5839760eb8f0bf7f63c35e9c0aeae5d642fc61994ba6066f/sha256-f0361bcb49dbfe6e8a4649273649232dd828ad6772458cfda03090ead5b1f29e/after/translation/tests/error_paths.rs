//! Phase C — error/rejection-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. `synth_pair` returns `void`, so the
//! "error code" the C reports is the *sentinel value it stores*: `32767`
//! (`INT16_MAX`) for the upper clamp and `-32768` (`INT16_MIN`) for the lower
//! clamp. Each test therefore asserts BOTH that the two `.so`s agree AND that
//! the exact documented sentinel was produced — not merely that "both did
//! something".

mod common;

use common::*;

/// `z[7*64] * 75038` is the last term of accumulator 1; 75038 is exactly
/// representable in `f32`.
const C1: f32 = 75038.0;
/// `(z+2)[8*64] * 64019` is a term of accumulator 2.
const C2: f32 = 64019.0;

fn zeros() -> Vec<f32> {
    vec![0.0f32; Z_LEN]
}

/// Build a `z` that drives accumulator 1 to `a1` and accumulator 2 to `a2`.
fn drive(a1: f32, a2: f32) -> Vec<f32> {
    let mut z = zeros();
    z[tap1(7)] = a1 / C1;
    z[tap2(8)] = a2 / C2;
    z
}

/// Call both `.so`s, assert byte equality, and return `(pcm[0], pcm[16*nch])`
/// as observed from the C library.
#[track_caller]
fn both(nch: i32, z: &[f32], what: &str) -> (i16, i16) {
    assert_same(&Pair::load(), nch, z, what);

    let pair = Pair::load();
    let mut c_pcm = Pcm::new();
    let mut rs_pcm = Pcm::new();
    unsafe {
        (pair.c)(c_pcm.base(), nch, z.as_ptr());
        (pair.rs)(rs_pcm.base(), nch, z.as_ptr());
    }
    assert_eq!(c_pcm.as_slice(), rs_pcm.as_slice(), "buffers differ [{what}]");

    let off = store_offsets(nch);
    let idx0 = (Pcm::HALF as isize + off[0]) as usize;
    let idx1 = (Pcm::HALF as isize + off[1]) as usize;
    (c_pcm.as_slice()[idx0], c_pcm.as_slice()[idx1])
}

// ------------------------------------------------------------------ rows 1-2
#[test]
fn err01_upper_clamp_store0() {
    // a >= 32766.5 on accumulator 1 -> INT16_MAX sentinel.
    for &a in &[32766.5f32, 40000.0, 1.0e6, 1.0e30, f32::MAX] {
        let z = drive(a, 0.0);
        let (p0, _) = both(1, &z, &format!("err01 a1={a}"));
        assert_eq!(p0, 32767, "upper clamp sentinel for a1={a}");
    }
}

#[test]
fn err02_upper_clamp_store_nch() {
    for &a in &[32766.5f32, 40000.0, 1.0e6, 1.0e30, f32::MAX] {
        let z = drive(0.0, a);
        let (_, p1) = both(1, &z, &format!("err02 a2={a}"));
        assert_eq!(p1, 32767, "upper clamp sentinel for a2={a}");
    }
}

// ------------------------------------------------------------------ rows 3-4
#[test]
fn err03_lower_clamp_store0() {
    for &a in &[-32767.5f32, -40000.0, -1.0e6, -1.0e30, f32::MIN] {
        let z = drive(a, 0.0);
        let (p0, _) = both(1, &z, &format!("err03 a1={a}"));
        assert_eq!(p0, -32768, "lower clamp sentinel for a1={a}");
    }
}

#[test]
fn err04_lower_clamp_store_nch() {
    for &a in &[-32767.5f32, -40000.0, -1.0e6, -1.0e30, f32::MIN] {
        let z = drive(0.0, a);
        let (_, p1) = both(1, &z, &format!("err04 a2={a}"));
        assert_eq!(p1, -32768, "lower clamp sentinel for a2={a}");
    }
}

// -------------------------------------------------------------------- row 5
#[test]
fn err05_upper_boundary_is_inclusive() {
    // `sample >= 32766.5` -- exactly on the boundary must CLAMP.
    let z = drive(32766.5, 32766.5);
    let (p0, p1) = both(1, &z, "err05 exactly +32766.5");
    assert_eq!(p0, 32767, "a == 32766.5 must clamp (>= is inclusive)");
    assert_eq!(p1, 32767, "a == 32766.5 must clamp on store 2");
}

// -------------------------------------------------------------------- row 6
#[test]
fn err06_lower_boundary_is_inclusive() {
    let z = drive(-32767.5, -32767.5);
    let (p0, p1) = both(1, &z, "err06 exactly -32767.5");
    assert_eq!(p0, -32768, "a == -32767.5 must clamp (<= is inclusive)");
    assert_eq!(p1, -32768, "a == -32767.5 must clamp on store 2");
}

// -------------------------------------------------------------------- row 7
/// Find the tap value whose `f32` product with `c` is the LARGEST value still
/// strictly below `bound` (stepping in `f32` ULPs, so the accumulator really
/// lands one representable step inside the clamp rather than approximately).
fn tap_just_below(c: f32, bound: f32) -> (f32, f32) {
    let mut t = bound / c;
    // Walk down until the product is below the bound.
    while t * c >= bound {
        t = f32::from_bits(t.to_bits() - 1);
    }
    // Then walk back up as far as possible while staying below.
    loop {
        let up = f32::from_bits(t.to_bits() + 1);
        if up * c < bound {
            t = up;
        } else {
            break;
        }
    }
    (t, t * c)
}

/// Mirror of `tap_just_below` for the negative bound: the product must be the
/// value closest to `bound` while still strictly greater than it.
fn tap_just_above(c: f32, bound: f32) -> (f32, f32) {
    let mut t = bound / c;
    while t * c <= bound {
        // move toward zero
        t = f32::from_bits(t.to_bits() - 1);
    }
    loop {
        let down = f32::from_bits(t.to_bits() + 1);
        if down * c > bound {
            t = down;
        } else {
            break;
        }
    }
    (t, t * c)
}

#[test]
fn err07_one_step_below_upper_boundary_does_not_clamp() {
    let pair = Pair::load();

    // Accumulator 1 via z[7*64] * 75038.
    let (t1, a1) = tap_just_below(C1, 32766.5);
    assert!(a1 < 32766.5, "a1={a1} must stay below the clamp boundary");
    assert!(a1 > 32765.0, "a1={a1} must be right at the boundary, got too far");
    let mut z = zeros();
    z[tap1(7)] = t1;
    assert_same(&pair, 1, &z, "err07 acc1 just below +32766.5");
    let mut pcm = Pcm::new();
    unsafe { (pair.c)(pcm.base(), 1, z.as_ptr()) };
    assert_eq!(
        pcm.as_slice()[Pcm::HALF], 32766,
        "just-below-boundary must NOT clamp: trunc(a + .5f) with no decrement"
    );

    // Accumulator 2 via (z+2)[8*64] * 64019.
    let (t2, a2) = tap_just_below(C2, 32766.5);
    assert!(a2 < 32766.5 && a2 > 32765.0, "a2={a2}");
    let mut z = zeros();
    z[tap2(8)] = t2;
    assert_same(&pair, 1, &z, "err07 acc2 just below +32766.5");
    let mut pcm = Pcm::new();
    unsafe { (pair.c)(pcm.base(), 1, z.as_ptr()) };
    assert_eq!(pcm.as_slice()[Pcm::HALF + 16], 32766, "same on store 2");

    // The exact f32 predecessor of 32766.5 as a value: 32766.498046875.
    let pred = f32::from_bits(32766.5f32.to_bits() - 1);
    assert_eq!(pred, 32766.498046875, "expected the f32 predecessor of 32766.5");
}

// -------------------------------------------------------------------- row 8
#[test]
fn err08_one_step_above_lower_boundary_does_not_clamp() {
    let pair = Pair::load();

    let (t1, a1) = tap_just_above(C1, -32767.5);
    assert!(a1 > -32767.5 && a1 < -32766.0, "a1={a1}");
    let mut z = zeros();
    z[tap1(7)] = t1;
    assert_same(&pair, 1, &z, "err08 acc1 just above -32767.5");
    let mut pcm = Pcm::new();
    unsafe { (pair.c)(pcm.base(), 1, z.as_ptr()) };
    assert_eq!(
        pcm.as_slice()[Pcm::HALF], -32767,
        "just-above-boundary must NOT clamp: trunc gives -32766 then the s<0 decrement"
    );

    let (t2, a2) = tap_just_above(C2, -32767.5);
    assert!(a2 > -32767.5 && a2 < -32766.0, "a2={a2}");
    let mut z = zeros();
    z[tap2(8)] = t2;
    assert_same(&pair, 1, &z, "err08 acc2 just above -32767.5");
    let mut pcm = Pcm::new();
    unsafe { (pair.c)(pcm.base(), 1, z.as_ptr()) };
    assert_eq!(pcm.as_slice()[Pcm::HALF + 16], -32767, "same on store 2");

    let succ = f32::from_bits((-32767.5f32).to_bits() - 1);
    assert_eq!(succ, -32767.498046875, "expected the f32 successor of -32767.5");
}

// -------------------------------------------------------------------- row 9
#[test]
fn err09_small_negative_does_not_decrement() {
    // a in (-0.5, 0): a + .5f in (0, 0.5) -> trunc 0 -> s<0 false -> 0.
    for &a in &[-0.25f32, -0.125, -1.0e-6, -f32::MIN_POSITIVE] {
        let z = drive(a, a);
        let (p0, p1) = both(1, &z, &format!("err09 a={a:e}"));
        assert_eq!(p0, 0, "small negative a={a:e} must yield 0, not -1");
        assert_eq!(p1, 0, "same on store 2 for a={a:e}");
    }
}

// ------------------------------------------------------------------- row 10
#[test]
fn err10_negative_zero_accumulator() {
    // Force a genuine -0.0f accumulator: 0.0 * -5 == -0.0, and the first term
    // of accumulator 1 is (z[14*64] - z[0]) * 29 with both taps 0 -> +0.0.
    let mut z = zeros();
    z[tap2(0)] = 0.0; // -> 0.0 * -5 == -0.0 as the LAST add of accumulator 2
    let (p0, p1) = both(1, &z, "err10 signed zero");
    assert_eq!(p0, 0, "+0.0 accumulator -> 0");
    assert_eq!(p1, 0, "-0.0 accumulator -> 0 (not -1)");

    // And explicitly a -0.0 valued tap.
    let mut z = zeros();
    z[tap1(7)] = -0.0;
    z[tap2(8)] = -0.0;
    let (p0, p1) = both(1, &z, "err10 -0.0 taps");
    assert_eq!((p0, p1), (0, 0));
}

// ------------------------------------------------------------------- row 11
#[test]
fn err11_minus_one_truncates_to_zero() {
    // a == -1.0: -1.0 + .5f == -0.5f, truncates TOWARD ZERO to 0, s<0 false.
    for &a in &[-1.0f32, -0.75, -0.5] {
        let z = drive(a, a);
        let (p0, p1) = both(1, &z, &format!("err11 a={a}"));
        assert_eq!(p0, 0, "a={a} must yield 0, not -1");
        assert_eq!(p1, 0, "a={a} must yield 0 on store 2");
    }
    // Just past that window: a == -1.5 -> -1.0f -> trunc -1 -> decrement -> -2.
    let z = drive(-1.5, -1.5);
    let (p0, p1) = both(1, &z, "err11 a=-1.5");
    assert_eq!(p0, -2, "a=-1.5 -> trunc(-1.0) = -1, then s<0 decrement -> -2");
    assert_eq!(p1, -2);
}

// ------------------------------------------------------------------- row 12
#[test]
fn err12_nan_accumulator_reaches_the_cast() {
    // Both comparisons are false for NaN, so the (int16_t) cast of NaN runs.
    // Construct NaN three ways: a NaN tap, and +Inf + (-Inf) via the
    // subtraction pairs of accumulator 1.
    let mut z = zeros();
    z[tap1(7)] = f32::NAN;
    let (p0, _) = both(1, &z, "err12 NaN tap");
    assert_eq!(p0, 0, "NaN must scale to 0 (cvttss2si -> 0x80000000, low 16 = 0)");

    // Inf - Inf inside `(z[14*64] - z[0]) * 29`.
    let mut z = zeros();
    z[tap1(14)] = f32::INFINITY;
    z[tap1(0)] = f32::INFINITY;
    let (p0, _) = both(1, &z, "err12 Inf-Inf -> NaN");
    assert_eq!(p0, 0, "Inf - Inf == NaN must scale to 0");

    // Inf + -Inf across two additive terms.
    let mut z = zeros();
    z[tap1(1)] = f32::INFINITY;
    z[tap1(3)] = f32::NEG_INFINITY;
    let (p0, _) = both(1, &z, "err12 +Inf then -Inf -> NaN");
    assert_eq!(p0, 0);

    // Negative NaN and a signalling-ish payload.
    for bits in [0xFFC0_0000u32, 0x7FC0_0001, 0x7F80_0001, 0xFF80_0001] {
        let mut z = zeros();
        z[tap1(7)] = f32::from_bits(bits);
        z[tap2(8)] = f32::from_bits(bits);
        let (p0, p1) = both(1, &z, &format!("err12 NaN bits {bits:#010x}"));
        assert_eq!((p0, p1), (0, 0), "NaN payload {bits:#010x}");
    }
}

// ------------------------------------------------------------------- row 13
#[test]
fn err13_positive_infinity_takes_upper_clamp() {
    let mut z = zeros();
    z[tap1(7)] = f32::INFINITY; // * 75038 -> +Inf
    z[tap2(8)] = f32::INFINITY;
    let (p0, p1) = both(1, &z, "err13 +Inf");
    assert_eq!(p0, 32767, "+Inf >= 32766.5 -> INT16_MAX");
    assert_eq!(p1, 32767, "+Inf on store 2 -> INT16_MAX");
}

// ------------------------------------------------------------------- row 14
#[test]
fn err14_negative_infinity_takes_lower_clamp() {
    let mut z = zeros();
    z[tap1(7)] = f32::NEG_INFINITY;
    z[tap2(8)] = f32::NEG_INFINITY;
    let (p0, p1) = both(1, &z, "err14 -Inf");
    assert_eq!(p0, -32768, "-Inf <= -32767.5 -> INT16_MIN");
    assert_eq!(p1, -32768, "-Inf on store 2 -> INT16_MIN");
}

// ------------------------------------------------------------------- row 15
#[test]
fn err15_nch_zero_aliases_stores_second_wins() {
    let z = drive(1.0e6, -1.0e6); // store0 clamps high, store1 clamps low
    let pair = Pair::load();
    let mut c_pcm = Pcm::new();
    let mut rs_pcm = Pcm::new();
    unsafe {
        (pair.c)(c_pcm.base(), 0, z.as_ptr());
        (pair.rs)(rs_pcm.base(), 0, z.as_ptr());
    }
    assert_eq!(c_pcm.as_slice(), rs_pcm.as_slice(), "nch=0 buffers differ");
    assert_eq!(
        c_pcm.as_slice()[Pcm::HALF],
        -32768,
        "at nch=0 the SECOND store must win (C)"
    );
    assert_eq!(
        rs_pcm.as_slice()[Pcm::HALF],
        -32768,
        "at nch=0 the SECOND store must win (Rust)"
    );

    // The reverse ordering, to prove it is order and not value.
    let z = drive(-1.0e6, 1.0e6);
    let mut c_pcm = Pcm::new();
    let mut rs_pcm = Pcm::new();
    unsafe {
        (pair.c)(c_pcm.base(), 0, z.as_ptr());
        (pair.rs)(rs_pcm.base(), 0, z.as_ptr());
    }
    assert_eq!(c_pcm.as_slice()[Pcm::HALF], 32767);
    assert_eq!(rs_pcm.as_slice()[Pcm::HALF], 32767);
}

// ------------------------------------------------------------------- row 16
#[test]
fn err16_negative_nch_writes_before_pcm() {
    let pair = Pair::load();
    let z = drive(500.25, -700.75);
    for nch in [-1i32, -2, -5, -100] {
        let mut c_pcm = Pcm::new();
        let mut rs_pcm = Pcm::new();
        unsafe {
            (pair.c)(c_pcm.base(), nch, z.as_ptr());
            (pair.rs)(rs_pcm.base(), nch, z.as_ptr());
        }
        assert_eq!(c_pcm.as_slice(), rs_pcm.as_slice(), "nch={nch} buffers differ");

        let neg = (Pcm::HALF as isize + (nch as isize) * 16) as usize;
        assert_ne!(
            c_pcm.as_slice()[neg],
            0x5A5Au16 as i16,
            "nch={nch}: C must write pcm[{}]",
            nch * 16
        );
        let pos = (Pcm::HALF as isize - (nch as isize) * 16) as usize;
        assert_eq!(
            c_pcm.as_slice()[pos],
            0x5A5Au16 as i16,
            "nch={nch}: nothing may be written at the mirrored positive slot"
        );
    }
}

// ------------------------------------------------------------------- row 17
#[test]
fn err17_nch_extremes_wrap_16x_in_int32() {
    // `16 * nch` is computed in `int` (GCC: `shl $0x4,%eax; cltq`), so it wraps
    // mod 2^32 and is THEN sign-extended. A 64-bit multiply would land
    // somewhere else entirely.
    let pair = Pair::load();
    let z = drive(1.0e6, -1.0e6); // distinguishable sentinels: 32767 / -32768

    let cases: [(i32, isize); 6] = [
        (i32::MAX, -16),
        (i32::MIN, 0),
        (i32::MAX - 1, -32),
        (i32::MIN + 1, 16),
        (0x0FFF_FFFF, -16),
        (0x7FFF_FFF0, -256),
    ];

    for (nch, expected_off) in cases {
        assert_eq!(
            nch.wrapping_mul(16) as isize,
            expected_off,
            "wrap model wrong for nch={nch}"
        );

        let mut c_pcm = Pcm::new();
        let mut rs_pcm = Pcm::new();
        unsafe {
            (pair.c)(c_pcm.base(), nch, z.as_ptr());
            (pair.rs)(rs_pcm.base(), nch, z.as_ptr());
        }
        assert_eq!(c_pcm.as_slice(), rs_pcm.as_slice(), "nch={nch} buffers differ");

        // Prove the C really wrote at the WRAPPED offset.
        let idx = (Pcm::HALF as isize + expected_off) as usize;
        let expected = if expected_off == 0 { -32768 } else { -32768 };
        assert_eq!(
            c_pcm.as_slice()[idx], expected,
            "nch={nch}: C must store the second sample at offset {expected_off}"
        );
    }
}

// ---------------------------------------------------------------- rows 18-19
#[test]
fn err18_19_null_pointers_are_undefined_and_not_dereferenced() {
    // `synth_pair` performs an unconditional load from `z` and store to `pcm`;
    // there is no null check anywhere in `c_src/src/lib.c` (grep for `NULL`
    // returns nothing). Passing NULL is UB in the C and faults in both
    // implementations, so it cannot be compared differentially in-process.
    // What IS verifiable is that neither library exports any validation entry
    // point and that both are equally unconditional -- asserted structurally
    // here so the row is covered rather than silently skipped.
    let c_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    assert!(
        !c_src.contains("NULL") && !c_src.contains("assert"),
        "the C gained a null/assert check; ERRORS.md rows 18-19 must be re-derived"
    );

    let rs_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    // The Rust must NOT have invented a null check the C does not have (that
    // would make it return early where the C faults).
    for pat in ["is_null", "assert!", "unwrap()", "expect(", "unimplemented", "todo!"] {
        assert!(
            !rs_src.contains(pat),
            "Rust added `{pat}` -- behaviour would diverge from the unconditional C"
        );
    }
}

// ------------------------------------------------------------------- row 20
#[test]
fn err20_minimum_z_length_no_overread() {
    // Guarded so that reading z[899] faults. Covered structurally in Phase B
    // row 17 as well; repeated here as the ERRORS.md row's own test.
    let pair = Pair::load();
    let mut z = vec![0.0f32; Z_LEN];
    for (i, v) in z.iter_mut().enumerate() {
        *v = (i as f32) * 0.125 - 50.0;
    }
    assert_same(&pair, 1, &z, "err20 exactly Z_LEN floats");
    assert_eq!(Z_LEN, 899, "z's minimum in-bounds length");
}

// ------------------------------------------------------------------- row 21
#[test]
fn err21_all_zero_taps_yield_zero() {
    let z = zeros();
    let (p0, p1) = both(1, &z, "err21 all zeros");
    assert_eq!((p0, p1), (0, 0), "all-zero input must produce (0, 0)");
}

// ------------------------------------------------------------------- row 22
#[test]
fn err22_subnormal_taps() {
    let pair = Pair::load();
    for &t in &[f32::MIN_POSITIVE, -f32::MIN_POSITIVE, f32::from_bits(1), f32::from_bits(0x8000_0001)]
    {
        let mut z = zeros();
        for v in z.iter_mut() {
            *v = t;
        }
        assert_same(&pair, 1, &z, &format!("err22 subnormal {t:e}"));

        let mut c_pcm = Pcm::new();
        unsafe { (pair.c)(c_pcm.base(), 1, z.as_ptr()) };
        assert_eq!(c_pcm.as_slice()[Pcm::HALF], 0, "subnormal accumulator -> 0");
        assert_eq!(c_pcm.as_slice()[Pcm::HALF + 16], 0, "subnormal accumulator -> 0");
    }
}

// ---------------------------------------------- generic FFI boundary sweep
#[test]
fn generic_nch_full_domain_sample() {
    // The C API has no enum; `int nch` accepts every 32-bit value, and `16*nch`
    // wraps mod 2^32. Because `nch = m * 2^28 + s` gives `16*nch mod 2^32 ==
    // 16*s`, sampling `m` over all 16 high nibbles and `s` over a small signed
    // range covers the ENTIRE `int` domain while keeping every store inside the
    // scratch buffer.
    let pair = Pair::load();
    let mut rng = Rng::new(0xFFFF_0000_1234_5678);
    let z = drive(12345.5, -6789.25);

    let mut tested = 0usize;
    for m in 0..16i64 {
        for _ in 0..64 {
            let s = (rng.next_u32() % 501) as i64 - 250; // -250 ..= 250
            let nch = (m.wrapping_mul(0x1000_0000).wrapping_add(s) & 0xFFFF_FFFF) as u32 as i32;
            assert_eq!(
                nch.wrapping_mul(16) as isize,
                (16 * s) as isize,
                "wrap model wrong for m={m} s={s} nch={nch}"
            );
            assert!(nch_fits(nch));
            assert_same(&pair, nch, &z, &format!("generic nch={nch} (m={m}, s={s})"));
            tested += 1;
        }
    }

    // Plus purely random values that happen to fit.
    for _ in 0..200_000 {
        let nch = rng.next_u32() as i32;
        if !nch_fits(nch) {
            continue;
        }
        assert_same(&pair, nch, &z, &format!("generic random nch={nch}"));
        tested += 1;
    }
    assert!(tested > 1000, "expected many nch samples, got {tested}");
}

#[test]
fn generic_zero_and_boundary_lengths() {
    // Oversized z: the library must still read only the first 899 floats, so
    // a huge buffer with garbage past index 898 must not change the result.
    let pair = Pair::load();
    let mut rng = Rng::new(0xABCD_0000_0000_0001);
    let mut small = vec![0.0f32; Z_LEN];
    for v in small.iter_mut() {
        *v = rng.sym(32.0);
    }
    let mut big = small.clone();
    big.resize(Z_LEN * 4, 0.0);
    let mut rng2 = Rng::new(0x9999);
    for v in big[Z_LEN..].iter_mut() {
        *v = rng2.any_f32();
    }

    for nch in [1i32, 2, 0, -1] {
        let mut a = Pcm::new();
        let mut b = Pcm::new();
        unsafe {
            (pair.c)(a.base(), nch, small.as_ptr());
            (pair.c)(b.base(), nch, big.as_ptr());
        }
        assert_eq!(a.as_slice(), b.as_slice(), "C must ignore z past index 898");

        let mut a = Pcm::new();
        let mut b = Pcm::new();
        unsafe {
            (pair.rs)(a.base(), nch, small.as_ptr());
            (pair.rs)(b.base(), nch, big.as_ptr());
        }
        assert_eq!(a.as_slice(), b.as_slice(), "Rust must ignore z past index 898");
    }
}
