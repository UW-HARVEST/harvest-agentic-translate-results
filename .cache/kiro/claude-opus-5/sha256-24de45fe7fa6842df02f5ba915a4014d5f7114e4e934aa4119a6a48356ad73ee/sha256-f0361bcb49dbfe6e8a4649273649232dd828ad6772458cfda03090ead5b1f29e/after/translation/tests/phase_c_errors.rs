//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row.
//!
//! The C API has no error-return channel (see the grep table in `ERRORS.md`:
//! zero `assert`s, zero error sentinels, zero pointers, zero allocations), so
//! the rejection surface consists of the degenerate IEEE-754 sentinels the C
//! produces without guarding, the exact range boundaries of the transfer
//! function, and the generic FFI boundary conditions. Each test asserts the two
//! implementations agree on the SAME sentinel bit pattern, not merely that both
//! "did something".

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — B is pure black: Low == 0, High > 0, unguarded division -> +inf.
// ---------------------------------------------------------------------------

#[test]
fn err01_zero_denominator_b_black() {
    let p = load();
    let mut rng = Rng::for_row(101);
    let mut checked = 0usize;
    for _ in 0..samples(30_000) {
        let a = rng.rgb();
        if a == BLACK {
            continue; // that is row 3
        }
        let cv = p.c.call(a, BLACK);
        let rv = p.rust.call(a, BLACK);
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "err01 divergence A=({},{},{}) vs black: C={cv:?} Rust={rv:?}",
            a.r,
            a.g,
            a.b
        );
        assert_eq!(
            cv.to_bits(),
            f32::INFINITY.to_bits(),
            "err01 expected the exact +inf sentinel 0x7F800000, got 0x{:08X}",
            cv.to_bits()
        );
        checked += 1;
    }
    // Every single non-black color, exhaustively on the darkest shell where the
    // numerator is smallest (most likely place for a different sentinel).
    for r in 0..=2u8 {
        for g in 0..=2u8 {
            for b in 0..=2u8 {
                let a = Rgb::new(r, g, b);
                if a == BLACK {
                    continue;
                }
                p.assert_same(a, BLACK, "err01 dark shell vs black");
                assert_eq!(p.c.call(a, BLACK).to_bits(), f32::INFINITY.to_bits());
                checked += 1;
            }
        }
    }
    assert!(checked > 0, "err01 checked nothing");
}

// ---------------------------------------------------------------------------
// Row 2 — A is pure black: swap taken, then Low == 0 -> +inf.
// ---------------------------------------------------------------------------

#[test]
fn err02_zero_denominator_a_black_after_swap() {
    let p = load();
    let mut rng = Rng::for_row(102);
    for _ in 0..samples(30_000) {
        let b = rng.rgb();
        if b == BLACK {
            continue;
        }
        let cv = p.c.call(BLACK, b);
        let rv = p.rust.call(BLACK, b);
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "err02 divergence black vs B=({},{},{}): C={cv:?} Rust={rv:?}",
            b.r,
            b.g,
            b.b
        );
        assert_eq!(
            cv.to_bits(),
            f32::INFINITY.to_bits(),
            "err02 expected the exact +inf sentinel"
        );
    }
    for &c in CORNERS.iter().filter(|&&c| c != BLACK) {
        p.assert_same(BLACK, c, "err02 black vs corner");
        assert_eq!(p.c.call(BLACK, c).to_bits(), f32::INFINITY.to_bits());
    }
}

// ---------------------------------------------------------------------------
// Row 3 — both black: 0.0f / 0.0f -> NaN, same bits on both sides.
// ---------------------------------------------------------------------------

#[test]
fn err03_zero_over_zero_is_nan() {
    let p = load();
    let cv = p.c.call(BLACK, BLACK);
    let rv = p.rust.call(BLACK, BLACK);
    assert!(cv.is_nan(), "err03 C did not produce NaN: {cv:?}");
    assert!(rv.is_nan(), "err03 Rust did not produce NaN: {rv:?}");
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "err03 NaN bit patterns differ: C=0x{:08X} Rust=0x{:08X}",
        cv.to_bits(),
        rv.to_bits()
    );
    assert_eq!(
        cv.is_sign_negative(),
        rv.is_sign_negative(),
        "err03 NaN sign differs"
    );
    // Repeat many times: the sentinel must be stable, not incidental.
    for _ in 0..1000 {
        assert_eq!(p.c.call(BLACK, BLACK).to_bits(), cv.to_bits());
        assert_eq!(p.rust.call(BLACK, BLACK).to_bits(), cv.to_bits());
    }
}

// ---------------------------------------------------------------------------
// Row 4 — equal non-black colors: High == Low, `<` is false, exactly 1.0.
// ---------------------------------------------------------------------------

#[test]
fn err04_equal_colors_exactly_one() {
    let p = load();
    let mut rng = Rng::for_row(104);
    for _ in 0..samples(30_000) {
        let a = rng.rgb();
        if a == BLACK {
            continue;
        }
        let cv = p.c.call(a, a);
        let rv = p.rust.call(a, a);
        assert_eq!(cv.to_bits(), rv.to_bits(), "err04 divergence for A==B");
        assert_eq!(
            cv.to_bits(),
            1.0f32.to_bits(),
            "err04 expected exactly 1.0 for A==B=({},{},{}), got {cv:?}",
            a.r,
            a.g,
            a.b
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 — the equality path AND the 0/0 path at once: A == B == black must be
// NaN, NOT the 1.0 an "equal colors" shortcut would return.
// ---------------------------------------------------------------------------

#[test]
fn err05_equal_black_is_nan_not_one() {
    let p = load();
    let cv = p.c.call(BLACK, BLACK);
    let rv = p.rust.call(BLACK, BLACK);
    assert_ne!(
        cv.to_bits(),
        1.0f32.to_bits(),
        "err05 C unexpectedly special-cases equal colors"
    );
    assert_ne!(
        rv.to_bits(),
        1.0f32.to_bits(),
        "err05 Rust wrongly special-cases equal colors and returns 1.0 where the C returns NaN"
    );
    assert_eq!(cv.to_bits(), rv.to_bits(), "err05 divergence for black==black");
}

// ---------------------------------------------------------------------------
// Row 6 — smallest non-zero denominator: no spurious overflow to inf.
// ---------------------------------------------------------------------------

#[test]
fn err06_smallest_nonzero_denominator_stays_finite() {
    let p = load();
    // Every color one step away from black, against white: the largest finite
    // ratios the library can produce.
    let darkest = [
        Rgb::new(1, 0, 0),
        Rgb::new(0, 1, 0),
        Rgb::new(0, 0, 1),
        Rgb::new(1, 1, 1),
    ];
    for &d in &darkest {
        p.assert_same(d, WHITE, "err06 darkest vs white");
        p.assert_same(WHITE, d, "err06 white vs darkest");
        let cv = p.c.call(d, WHITE);
        assert!(
            cv.is_finite(),
            "err06 C produced {cv:?} for ({},{},{}) vs white; expected a finite ratio",
            d.r,
            d.g,
            d.b
        );
        assert_eq!(p.rust.call(d, WHITE).is_finite(), cv.is_finite());
    }
}

// ---------------------------------------------------------------------------
// Rows 7-8 — the transfer-function threshold: byte 10 is the last linear value,
// byte 11 is one step past it and takes `pow`.
// ---------------------------------------------------------------------------

#[test]
fn err07_threshold_low_side_byte_10_linear() {
    let p = load();
    // 10/255 = 0.0392156... <= 0.04045
    assert!((10.0f32 / 255.0f32) as f64 <= 0.04045);
    for pos in 0..3 {
        for partner in [WHITE, MIDGRAY, Rgb::new(11, 11, 11)] {
            let a = match pos {
                0 => Rgb::new(LINEAR_MAX, 0, 0),
                1 => Rgb::new(0, LINEAR_MAX, 0),
                _ => Rgb::new(0, 0, LINEAR_MAX),
            };
            p.assert_same(a, partner, "err07 byte-10 linear branch");
            p.assert_same(partner, a, "err07 byte-10 linear branch swapped");
        }
    }
    // Every value on the linear side of the threshold, all channels at once.
    for v in 0..=LINEAR_MAX {
        p.assert_same(Rgb::new(v, v, v), WHITE, "err07 linear sweep");
    }
}

#[test]
fn err08_threshold_high_side_byte_11_pow() {
    let p = load();
    // 11/255 = 0.0431372... > 0.04045
    assert!((11.0f32 / 255.0f32) as f64 > 0.04045);
    for pos in 0..3 {
        for partner in [WHITE, MIDGRAY, Rgb::new(10, 10, 10)] {
            let a = match pos {
                0 => Rgb::new(POW_MIN, 0, 0),
                1 => Rgb::new(0, POW_MIN, 0),
                _ => Rgb::new(0, 0, POW_MIN),
            };
            p.assert_same(a, partner, "err08 byte-11 pow branch");
            p.assert_same(partner, a, "err08 byte-11 pow branch swapped");
        }
    }
    // The two bytes bracketing the threshold, in every channel combination.
    for r in [LINEAR_MAX, POW_MIN] {
        for g in [LINEAR_MAX, POW_MIN] {
            for b in [LINEAR_MAX, POW_MIN] {
                p.assert_same(Rgb::new(r, g, b), WHITE, "err08 threshold cube");
                p.assert_same(Rgb::new(r, g, b), Rgb::new(1, 1, 1), "err08 threshold cube dark");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 9-10 — the minimum and maximum of the valid channel range.
// ---------------------------------------------------------------------------

#[test]
fn err09_channel_minimum_zero() {
    let p = load();
    let mut rng = Rng::for_row(109);
    for _ in 0..samples(10_000) {
        // Force one channel to the minimum, randomize the rest.
        let which = (rng.next_u64() % 3) as u8;
        let mut a = rng.rgb();
        match which {
            0 => a.r = 0,
            1 => a.g = 0,
            _ => a.b = 0,
        }
        p.assert_same(a, rng.rgb(), "err09 channel min 0");
    }
}

#[test]
fn err10_channel_maximum_255() {
    let p = load();
    let mut rng = Rng::for_row(110);
    for _ in 0..samples(10_000) {
        let which = (rng.next_u64() % 3) as u8;
        let mut a = rng.rgb();
        match which {
            0 => a.r = 255,
            1 => a.g = 255,
            _ => a.b = 255,
        }
        p.assert_same(a, rng.rgb(), "err10 channel max 255");
    }
    // pow(1.0, 2.4) must be exactly 1.0 on both sides.
    assert_eq!(
        p.c.call(WHITE, WHITE).to_bits(),
        p.rust.call(WHITE, WHITE).to_bits()
    );
    assert_eq!(p.c.call(WHITE, WHITE).to_bits(), 1.0f32.to_bits());
}

// ---------------------------------------------------------------------------
// Rows 11-13 — out-of-range bits across the FFI boundary. `cb_rgb_255` is 3
// bytes but travels in a full 8-byte SysV INTEGER register; the 5 unused high
// bytes are attacker/caller controlled. This is the analogue of passing an
// `enum` value with no valid variant: C accepts whatever is in the register and
// the Rust must interpret it identically.
// ---------------------------------------------------------------------------

#[test]
fn err11_out_of_range_register_bits_arg1() {
    let p = load();
    let mut rng = Rng::for_row(111);
    let garbage = [
        u64::MAX,
        0xFFFF_FFFF_FF00_0000,
        0xAAAA_AAAA_AA00_0000,
        0x8000_0000_0000_0000,
        0x0000_0000_FF00_0000,
    ];
    for &g in &garbage {
        for _ in 0..samples(2_000) {
            let (a, b) = (rng.rgb(), rng.rgb());
            p.assert_same_reg(a, b, g, 0, "err11 arg1 out-of-range bits");
        }
        for &a in &CORNERS {
            for &b in &CORNERS {
                p.assert_same_reg(a, b, g, 0, "err11 corners arg1");
            }
        }
    }
}

#[test]
fn err12_out_of_range_register_bits_arg2() {
    let p = load();
    let mut rng = Rng::for_row(112);
    let garbage = [
        u64::MAX,
        0xFFFF_FFFF_FF00_0000,
        0x5555_5555_5500_0000,
        0x8000_0000_0000_0000,
    ];
    for &g in &garbage {
        for _ in 0..samples(2_000) {
            let (a, b) = (rng.rgb(), rng.rgb());
            p.assert_same_reg(a, b, 0, g, "err12 arg2 out-of-range bits");
        }
        for &a in &CORNERS {
            for &b in &CORNERS {
                p.assert_same_reg(a, b, 0, g, "err12 corners arg2");
            }
        }
    }
}

#[test]
fn err13_out_of_range_register_bits_both() {
    let p = load();
    let mut rng = Rng::for_row(113);
    for _ in 0..samples(50_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        let (ga, gb) = (rng.next_u64(), rng.next_u64());
        p.assert_same_reg(a, b, ga, gb, "err13 both out-of-range bits");
    }
    // The degenerate sentinels must survive dirty padding unchanged.
    for &(a, b, want_nan) in &[
        (BLACK, BLACK, true),
        (WHITE, BLACK, false),
        (BLACK, WHITE, false),
    ] {
        for _ in 0..1000 {
            let (ga, gb) = (rng.next_u64(), rng.next_u64());
            p.assert_same_reg(a, b, ga, gb, "err13 degenerate + dirty");
            let v = p.c.call_reg(a.to_reg(ga), b.to_reg(gb));
            assert_eq!(v.is_nan(), want_nan, "err13 sentinel changed under dirty padding");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — null pointer / zero length / oversized length: NOT APPLICABLE.
// Asserted structurally so the class is accounted for rather than skipped.
// ---------------------------------------------------------------------------

#[test]
fn err14_no_pointer_or_length_surface_exists() {
    // `cb_rgb_255` is a 3-byte, alignment-1, pointer-free value type, and
    // `contrast_ratio` takes two of them BY VALUE and returns a bare `float`.
    // There is therefore no null pointer to pass, no length to zero out, and no
    // buffer to oversize.
    assert_eq!(std::mem::size_of::<Rgb>(), 3, "cb_rgb_255 must be 3 bytes");
    assert_eq!(std::mem::align_of::<Rgb>(), 1, "cb_rgb_255 must be align 1");

    // The C source contains no pointer parameter, no length parameter, and no
    // allocation; verified mechanically by grep in ERRORS.md. The only thing
    // left to check is that the value type really does round-trip its 3 bytes.
    let p = load();
    for r in [0u8, 1, 255] {
        for g in [0u8, 1, 255] {
            for b in [0u8, 1, 255] {
                let c = Rgb::new(r, g, b);
                // Same 3 bytes reached the callee on both sides.
                p.assert_same(c, WHITE, "err14 value-type round trip");
                p.assert_same_reg(c, WHITE, 0, 0, "err14 value-type round trip via reg");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — the result is never negative and never a negative zero: no input
// can make `Low` negative because every channel is unsigned.
// ---------------------------------------------------------------------------

#[test]
fn err15_result_is_never_negative() {
    let p = load();
    let mut rng = Rng::for_row(115);
    for _ in 0..samples(200_000) {
        let (a, b) = (rng.rgb(), rng.rgb());
        let cv = p.c.call(a, b);
        let rv = p.rust.call(a, b);
        assert_eq!(cv.to_bits(), rv.to_bits(), "err15 divergence");
        if !cv.is_nan() {
            assert!(
                cv >= 1.0f32,
                "err15 C produced {cv:?} for A=({},{},{}) B=({},{},{}); a High/Low ratio must be >= 1",
                a.r,
                a.g,
                a.b,
                b.r,
                b.g,
                b.b
            );
            assert!(!cv.is_sign_negative(), "err15 unexpected negative result");
            assert!(!rv.is_sign_negative(), "err15 unexpected negative result (Rust)");
        }
    }
}
