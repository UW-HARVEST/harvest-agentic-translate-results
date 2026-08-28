//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Every test loads BOTH `.so`s via `libloading` and compares their output
//! byte-for-byte. Randomized inputs use a fixed seed for reproducibility.

mod common;

use std::ffi::c_int;

use common::*;

/// How many randomized pixel buffers to push through each enumerated shape.
const REPS: usize = 32;

/// Drive one `(w, h)` shape with `REPS` random buffers plus the all-0x00 and
/// all-0xFF extremes.
fn sweep_shape(libs: &Libs, w: c_int, h: c_int, ctx: &str) {
    let n = (w.max(0) as usize) * (h.max(0) as usize);
    let mut rng = Rng::new(SEED ^ ((w as u64) << 32) ^ (h as u64 as u32 as u64));

    for rep in 0..REPS {
        let pixels = rng.pixels(n);
        let out = assert_same(libs, w, h, &pixels, &format!("{ctx} rep={rep}"));
        // Cross-check against an independent model of the C loop, so a
        // "both did nothing" result cannot silently pass as a match.
        assert_eq!(
            out.pixels,
            model(w, h, &pixels),
            "both libs agreed but disagree with the reference model ({ctx} rep={rep}) w={w} h={h}"
        );
    }

    for (label, fill) in [
        ("zeros", CpPixel { r: 0, g: 0, b: 0, a: 0 }),
        ("ones", CpPixel { r: 0xFF, g: 0xFF, b: 0xFF, a: 0xFF }),
    ] {
        let pixels = vec![fill; n];
        assert_same(libs, w, h, &pixels, &format!("{ctx} {label}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 1-2: fully degenerate
// ---------------------------------------------------------------------------

#[test]
fn row01_w0_h0_null_pix() {
    let libs = Libs::load();
    let c = run_one_null_pix(&libs, Impl::C, 0, 0);
    let r = run_one_null_pix(&libs, Impl::Rust, 0, 0);
    assert_eq!(c, r, "row 1: w=0 h=0 pix=NULL diverged");
    assert_eq!(c, (0, 0, true), "row 1: descriptor was mutated");
}

#[test]
fn row02_w0_h0_valid_empty_buffer() {
    let libs = Libs::load();
    sweep_shape(&libs, 0, 0, "row 2");
}

// ---------------------------------------------------------------------------
// Rows 3-5: zero / one row
// ---------------------------------------------------------------------------

#[test]
fn row03_w8_h0() {
    let libs = Libs::load();
    sweep_shape(&libs, 8, 0, "row 3");
}

#[test]
fn row04_w1_h1() {
    let libs = Libs::load();
    sweep_shape(&libs, 1, 1, "row 4");
}

#[test]
fn row05_w8_h1() {
    let libs = Libs::load();
    sweep_shape(&libs, 8, 1, "row 5");
}

// ---------------------------------------------------------------------------
// Rows 6-7: w == 0 with h >= 2 (outer loop spins, inner never runs), pix NULL
// ---------------------------------------------------------------------------

#[test]
fn row06_w0_h4_null_pix() {
    let libs = Libs::load();
    let c = run_one_null_pix(&libs, Impl::C, 0, 4);
    let r = run_one_null_pix(&libs, Impl::Rust, 0, 4);
    assert_eq!(c, r, "row 6: w=0 h=4 pix=NULL diverged");
    assert_eq!(c, (0, 4, true));
    // also with a real (empty) buffer
    sweep_shape(&libs, 0, 4, "row 6b");
}

#[test]
fn row07_w0_h5_null_pix() {
    let libs = Libs::load();
    let c = run_one_null_pix(&libs, Impl::C, 0, 5);
    let r = run_one_null_pix(&libs, Impl::Rust, 0, 5);
    assert_eq!(c, r, "row 7: w=0 h=5 pix=NULL diverged");
    assert_eq!(c, (0, 5, true));
    sweep_shape(&libs, 0, 5, "row 7b");
}

// ---------------------------------------------------------------------------
// Rows 8-14: real work, small shapes, both parities
// ---------------------------------------------------------------------------

#[test]
fn row08_w1_h2() {
    let libs = Libs::load();
    sweep_shape(&libs, 1, 2, "row 8");
}

#[test]
fn row09_w1_h3_middle_row_preserved() {
    let libs = Libs::load();
    sweep_shape(&libs, 1, 3, "row 9");

    // Explicitly assert the odd-h invariant the C has: row h/2 is untouched.
    let mut rng = Rng::new(SEED + 9);
    let pixels = rng.pixels(3);
    let out = assert_same(&libs, 1, 3, &pixels, "row 9 middle");
    assert_eq!(out.pixels[1], pixels[1], "middle row must be untouched");
    assert_eq!(out.pixels[0], pixels[2], "row 0 must become row 2");
    assert_eq!(out.pixels[2], pixels[0], "row 2 must become row 0");
}

#[test]
fn row10_w2_h2() {
    let libs = Libs::load();
    sweep_shape(&libs, 2, 2, "row 10");
}

#[test]
fn row11_w8_h2() {
    let libs = Libs::load();
    sweep_shape(&libs, 8, 2, "row 11");
}

#[test]
fn row12_w8_h3() {
    let libs = Libs::load();
    sweep_shape(&libs, 8, 3, "row 12");
}

#[test]
fn row13_w8_h4() {
    let libs = Libs::load();
    sweep_shape(&libs, 8, 4, "row 13");
}

#[test]
fn row14_w3_h5() {
    let libs = Libs::load();
    sweep_shape(&libs, 3, 5, "row 14");
}

// ---------------------------------------------------------------------------
// Rows 15-21: larger / lopsided shapes
// ---------------------------------------------------------------------------

#[test]
fn row15_w1_h64() {
    let libs = Libs::load();
    sweep_shape(&libs, 1, 64, "row 15");
}

#[test]
fn row16_w1_h65() {
    let libs = Libs::load();
    sweep_shape(&libs, 1, 65, "row 16");
}

#[test]
fn row17_w64_h1() {
    let libs = Libs::load();
    sweep_shape(&libs, 64, 1, "row 17");
}

#[test]
fn row18_w37_h64() {
    let libs = Libs::load();
    sweep_shape(&libs, 37, 64, "row 18");
}

#[test]
fn row19_w37_h65() {
    let libs = Libs::load();
    sweep_shape(&libs, 37, 65, "row 19");
}

#[test]
fn row20_w256_h2() {
    let libs = Libs::load();
    sweep_shape(&libs, 256, 2, "row 20");
}

#[test]
fn row21_w2_h256() {
    let libs = Libs::load();
    sweep_shape(&libs, 2, 256, "row 21");
}

// ---------------------------------------------------------------------------
// Row 22: same product, different factorisation -> different row addressing
// ---------------------------------------------------------------------------

#[test]
fn row22_same_product_different_factorisation() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 22);
    // One fixed 24-pixel buffer, reinterpreted under every factorisation.
    for rep in 0..REPS {
        let pixels = rng.pixels(24);
        for (w, h) in [
            (1, 24),
            (2, 12),
            (3, 8),
            (4, 6),
            (6, 4),
            (8, 3),
            (12, 2),
            (24, 1),
        ] {
            let out = assert_same(
                &libs,
                w,
                h,
                &pixels,
                &format!("row 22 {w}x{h} rep={rep}"),
            );
            assert_eq!(
                out.pixels,
                model(w, h, &pixels),
                "row 22 {w}x{h}: disagrees with reference model"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 23-25: data patterns
// ---------------------------------------------------------------------------

#[test]
fn row23_all_zero_pixels() {
    let libs = Libs::load();
    for &(w, h) in REPRESENTATIVE_SHAPES {
        let n = (w as usize) * (h as usize);
        let pixels = vec![CpPixel::default(); n];
        assert_same(&libs, w, h, &pixels, &format!("row 23 {w}x{h}"));
    }
}

#[test]
fn row24_all_ff_pixels() {
    let libs = Libs::load();
    for &(w, h) in REPRESENTATIVE_SHAPES {
        let n = (w as usize) * (h as usize);
        let pixels = vec![
            CpPixel { r: 0xFF, g: 0xFF, b: 0xFF, a: 0xFF };
            n
        ];
        assert_same(&libs, w, h, &pixels, &format!("row 24 {w}x{h}"));
    }
}

#[test]
fn row25_per_channel_distinguishable_pattern() {
    let libs = Libs::load();
    for &(w, h) in REPRESENTATIVE_SHAPES {
        let n = (w as usize) * (h as usize);
        // Every channel gets a different function of the index so that a
        // channel swap, an alpha drop, or a 3-byte copy is detectable.
        let pixels: Vec<CpPixel> = (0..n)
            .map(|i| {
                let i = i as u8;
                CpPixel {
                    r: i,
                    g: !i,
                    b: 0xA5,
                    a: i.wrapping_mul(7),
                }
            })
            .collect();
        let out = assert_same(&libs, w, h, &pixels, &format!("row 25 {w}x{h}"));
        assert_eq!(
            out.pixels,
            model(w, h, &pixels),
            "row 25 {w}x{h}: disagrees with reference model"
        );
        // Every pixel must still be a *whole* pixel from the input (no torn
        // channel-wise copies).
        for p in &out.pixels {
            assert!(
                pixels.contains(p),
                "row 25 {w}x{h}: produced a pixel {p:?} that was not in the input \
                 (channels were mixed)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26: involution — applying twice restores the original
// ---------------------------------------------------------------------------

#[test]
fn row26_double_application_is_identity() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 26);

    for &(w, h) in REPRESENTATIVE_SHAPES {
        let n = (w as usize) * (h as usize);
        for rep in 0..8 {
            let original = rng.pixels(n);
            for which in [Impl::C, Impl::Rust] {
                let once = run_one(&libs, which, w, h, &original);
                let twice = run_one(&libs, which, w, h, &once.pixels);
                assert_eq!(
                    twice.pixels, original,
                    "{} is not an involution at {w}x{h} rep={rep}",
                    which.name()
                );
            }
            // and the single-application results still agree
            assert_same(&libs, w, h, &original, &format!("row 26 {w}x{h} rep={rep}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 27-28: descriptor immutability + no out-of-bounds writes
//
// These invariants are asserted inside `assert_same` for EVERY row above; this
// test pins them down explicitly across a wide shape range.
// ---------------------------------------------------------------------------

#[test]
fn row27_row28_descriptor_and_bounds_respected() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 27);
    for w in 0..=12 {
        for h in 0..=12 {
            let n = (w as usize) * (h as usize);
            let pixels = rng.pixels(n);
            // assert_same checks: guard regions intact on both sides, w/h/pix
            // unchanged on both sides, and C == Rust for all of it.
            assert_same(&libs, w, h, &pixels, &format!("row 27/28 {w}x{h}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 29-30: randomized property sweeps
// ---------------------------------------------------------------------------

#[test]
fn row29_random_positive_shapes() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 29);
    for case in 0..400 {
        let w = rng.range_i32(1, 24);
        let h = rng.range_i32(1, 24);
        let pixels = rng.pixels((w as usize) * (h as usize));
        let out = assert_same(&libs, w, h, &pixels, &format!("row 29 case={case}"));
        assert_eq!(
            out.pixels,
            model(w, h, &pixels),
            "row 29 case={case} {w}x{h}: disagrees with reference model"
        );
    }
}

#[test]
fn row30_random_shapes_including_degenerate() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 30);
    for case in 0..400 {
        let w = rng.range_i32(-4, 12);
        let h = rng.range_i32(-4, 12);
        // Always hand over a real buffer sized for the *positive* interpretation
        // so that nothing can fault; when w or h is <= 0 no deref is due anyway.
        let n = (w.max(0) as usize) * (h.max(0) as usize);
        let pixels = rng.pixels(n);
        let out = assert_same(&libs, w, h, &pixels, &format!("row 30 case={case}"));
        assert_eq!(
            out.pixels,
            model(w, h, &pixels),
            "row 30 case={case} {w}x{h}: disagrees with reference model"
        );
    }
}

// ---------------------------------------------------------------------------
// Sanity: the tests are actually observing a flip, not a universal no-op.
// ---------------------------------------------------------------------------

#[test]
fn sanity_flip_actually_happens() {
    let libs = Libs::load();
    let pixels: Vec<CpPixel> = (0..6u8)
        .map(|i| CpPixel { r: i, g: i, b: i, a: i })
        .collect();
    // w=1, h=6 -> rows are single pixels, so this is a plain reversal.
    let out = assert_same(&libs, 1, 6, &pixels, "sanity");
    let reversed: Vec<CpPixel> = pixels.iter().rev().copied().collect();
    assert_eq!(
        out.pixels, reversed,
        "expected a full row reversal; the differential test would otherwise be \
         vacuous"
    );
    assert_ne!(out.pixels, pixels, "output must differ from input here");
}
