//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their exported `premultiply` symbol and compares the
//! resulting buffers byte-for-byte (including 64-byte poison guards).

mod common;

use common::*;

const SEED: u64 = 0x0DDB_A11C_0FFE_E123;

/// Row 1 — exhaustive: every channel value × every alpha, single pixel.
#[test]
fn row01_exhaustive_value_times_alpha() {
    let pair = load_pair();
    for a in 0u16..=255 {
        for v in 0u16..=255 {
            let px = [v as u8, v as u8, v as u8, a as u8];
            let out = diff_run(&pair, 1, 1, &px, "row01");
            // Cross-check against the reference model too.
            assert_eq!(
                out.as_slice(),
                &model_pixel(px)[..],
                "row01 model mismatch v={v} a={a}"
            );
        }
    }
}

/// Row 2 — exhaustive with three *different* channel values × every alpha.
#[test]
fn row02_exhaustive_distinct_channels() {
    let pair = load_pair();
    for a in 0u16..=255 {
        for v in 0u16..=255 {
            let px = [v as u8, (255 - v) as u8, (v / 2) as u8, a as u8];
            let out = diff_run(&pair, 1, 1, &px, "row02");
            assert_eq!(
                out.as_slice(),
                &model_pixel(px)[..],
                "row02 model mismatch v={v} a={a}"
            );
        }
    }
}

/// Row 3 — single pixel, 20000 seeded random values.
#[test]
fn row03_single_pixel_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED);
    for _ in 0..20_000 {
        let px = [rng.next_u8(), rng.next_u8(), rng.next_u8(), rng.next_u8()];
        diff_run(&pair, 1, 1, &px, "row03");
    }
}

/// Row 4 — w=256, h=1 ramp.
#[test]
fn row04_ramp_row() {
    let pair = load_pair();
    let mut data = vec![0u8; 256 * PIXEL_SIZE];
    for i in 0..256usize {
        data[i * 4] = i as u8;
        data[i * 4 + 1] = i as u8;
        data[i * 4 + 2] = i as u8;
        data[i * 4 + 3] = i as u8;
    }
    diff_run(&pair, 256, 1, &data, "row04");
}

/// Row 5 — 256x256 random (65536 px).
#[test]
fn row05_large_multirow_random() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..3 {
        let data = random_bytes(&mut rng, 256 * 256);
        diff_run(&pair, 256, 256, &data, "row05");
    }
}

/// Row 6 — w=2, h=3.
#[test]
fn row06_small_even_dims() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..500 {
        let data = random_bytes(&mut rng, 2 * 3);
        diff_run(&pair, 2, 3, &data, "row06");
    }
}

/// Row 7 — w=3, h=5 (odd, non-power-of-two).
#[test]
fn row07_odd_dims() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..500 {
        let data = random_bytes(&mut rng, 3 * 5);
        diff_run(&pair, 3, 5, &data, "row07");
    }
}

/// Row 8 — w=7, h=1 (single row, odd width).
#[test]
fn row08_single_row_odd_width() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..500 {
        let data = random_bytes(&mut rng, 7);
        diff_run(&pair, 7, 1, &data, "row08");
    }
}

/// Row 9 — w=1, h=7 (single column).
#[test]
fn row09_single_column() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..500 {
        let data = random_bytes(&mut rng, 7);
        diff_run(&pair, 1, 7, &data, "row09");
    }
}

/// Row 10 — 64x64 random.
#[test]
fn row10_medium_square() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..20 {
        let data = random_bytes(&mut rng, 64 * 64);
        diff_run(&pair, 64, 64, &data, "row10");
    }
}

/// Row 11 — w=1000, h=1 (wide row).
#[test]
fn row11_wide_row() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..20 {
        let data = random_bytes(&mut rng, 1000);
        diff_run(&pair, 1000, 1, &data, "row11");
    }
}

/// Row 12 — byte-misaligned `pix` (alignment of cp_pixel_t is 1, so legal).
#[test]
fn row12_misaligned_pix() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 12);
    for &offset in &[1usize, 2, 3, 5, 7] {
        for _ in 0..200 {
            let data = random_bytes(&mut rng, 5 * 3);
            diff_run_ex(&pair, 5, 3, &data, 64, offset, "row12");
        }
    }
}

/// Row 13 — shape equivalence: the C walks a flat byte run, so any (w,h) with
/// the same product must yield identical bytes.
#[test]
fn row13_shape_equivalence() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 13);
    const SHAPES: &[(i32, i32)] = &[
        (6, 4),
        (24, 1),
        (8, 3),
        (4, 6),
        (1, 24),
        (2, 12),
        (3, 8),
        (12, 2),
    ];
    for _ in 0..200 {
        let data = random_bytes(&mut rng, 24);
        let mut reference: Option<Vec<u8>> = None;
        for &(w, h) in SHAPES {
            assert_eq!(expected_iterations(w, h), 24, "shape {w}x{h} trip count");
            let out = diff_run(&pair, w, h, &data, "row13");
            match &reference {
                None => reference = Some(out),
                Some(r) => assert_eq!(
                    r, &out,
                    "row13 shape {w}x{h} differs from reference shape output"
                ),
            }
        }
    }
}

/// Row 14 — all-zero buffer.
#[test]
fn row14_all_zero() {
    let pair = load_pair();
    let data = vec![0u8; 16 * 4 * PIXEL_SIZE];
    let out = diff_run(&pair, 16, 4, &data, "row14");
    assert_eq!(out, data, "row14 all-zero must stay zero");
}

/// Row 15 — all-0xFF buffer (max round-trip).
#[test]
fn row15_all_ff() {
    let pair = load_pair();
    let data = vec![0xFFu8; 16 * 4 * PIXEL_SIZE];
    let out = diff_run(&pair, 16, 4, &data, "row15");
    assert_eq!(out, data, "row15 a=255,rgb=255 must round-trip to 255");
}

fn fixed_alpha_case(alpha: u8, tag: &str, seed_salt: u64) {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ seed_salt);
    let n = 32 * 2;
    for _ in 0..300 {
        let mut data = random_bytes(&mut rng, n);
        for i in 0..n {
            data[i * 4 + 3] = alpha;
        }
        let out = diff_run(&pair, 32, 2, &data, tag);
        // Alpha is read but never written back by the C.
        for i in 0..n {
            assert_eq!(out[i * 4 + 3], alpha, "{tag} alpha channel was modified");
        }
    }
}

/// Row 16 — alpha = 0 on every pixel: rgb must all become 0.
#[test]
fn row16_alpha_zero() {
    fixed_alpha_case(0, "row16", 16);
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x16);
    let mut data = random_bytes(&mut rng, 8);
    for i in 0..8 {
        data[i * 4 + 3] = 0;
    }
    let out = diff_run(&pair, 8, 1, &data, "row16b");
    for i in 0..8 {
        assert_eq!(&out[i * 4..i * 4 + 3], &[0, 0, 0], "row16 rgb must be zeroed");
    }
}

/// Row 17 — alpha = 255 (round-trip through /255 then *255 then truncate).
#[test]
fn row17_alpha_max() {
    fixed_alpha_case(255, "row17", 17);
}

/// Row 18 — alpha = 1 (smallest non-zero).
#[test]
fn row18_alpha_one() {
    fixed_alpha_case(1, "row18", 18);
}

/// Row 19 — alpha = 254 (one below max).
#[test]
fn row19_alpha_254() {
    fixed_alpha_case(254, "row19", 19);
}

/// Row 20 — alpha = 128 (mid-range).
#[test]
fn row20_alpha_mid() {
    fixed_alpha_case(128, "row20", 20);
}

/// Row 21 — rgb = 0 with random alpha: rgb stays 0, alpha untouched.
#[test]
fn row21_rgb_zero_random_alpha() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 21);
    let n = 32 * 2;
    for _ in 0..300 {
        let mut data = vec![0u8; n * PIXEL_SIZE];
        for i in 0..n {
            data[i * 4 + 3] = rng.next_u8();
        }
        let out = diff_run(&pair, 32, 2, &data, "row21");
        assert_eq!(out, data, "row21 rgb=0 must be a fixed point");
    }
}

/// Row 22 — repeated application (composed pipeline, not a single call).
#[test]
fn row22_repeated_application() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..100 {
        let data = random_bytes(&mut rng, 16 * 16);
        let once = diff_run(&pair, 16, 16, &data, "row22-pass1");
        let twice = diff_run(&pair, 16, 16, &once, "row22-pass2");
        let thrice = diff_run(&pair, 16, 16, &twice, "row22-pass3");
        // No assertion on the values themselves; the point is that both libs
        // agree at every stage of the composition.
        let _ = thrice;
    }
}

/// Row 23 — negative w AND negative h: bound is positive, loop runs.
#[test]
fn row23_both_dims_negative() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 23);
    const CASES: &[(i32, i32)] = &[(-1, -1), (-2, -3), (-4, -4), (-3, -5), (-8, -2)];
    for &(w, h) in CASES {
        let iters = expected_iterations(w, h);
        assert!(iters > 0, "row23 expects a running loop for {w}x{h}");
        for _ in 0..200 {
            let data = random_bytes(&mut rng, iters as usize);
            diff_run(&pair, w, h, &data, "row23");
        }
    }
}

/// Row 24 — w=INT_MAX, h=-1: bound wraps to exactly 4 → one pixel.
#[test]
fn row24_intmax_times_neg1() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 24);
    assert_eq!(expected_iterations(i32::MAX, -1), 1);
    for _ in 0..500 {
        // Give 8 pixels of live data; only the first must be touched.
        let data = random_bytes(&mut rng, 8);
        let out = diff_run(&pair, i32::MAX, -1, &data, "row24");
        assert_eq!(
            &out[4..],
            &data[4..],
            "row24 must touch only the first pixel"
        );
    }
}

/// Row 25 — w=65536, h=16385: bound overflows and wraps to a small positive
/// value (262144), so only 65536 of a nominal 2^30 pixels are processed.
#[test]
fn row25_overflow_to_small_positive() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 25);
    let w = 65536i32;
    let h = 16385i32;
    let iters = expected_iterations(w, h);
    assert_eq!(iters, 65536, "row25 expected wrapped trip count");
    for _ in 0..3 {
        // Live data is 2x the processed span so the untouched tail is checked.
        let data = random_bytes(&mut rng, iters as usize * 2);
        let out = diff_run(&pair, w, h, &data, "row25");
        let span = iters as usize * PIXEL_SIZE;
        assert_eq!(&out[span..], &data[span..], "row25 wrote past the wrapped end");
    }
}

/// Row 26 — both dims oversized: 0x7FFFFFFE x 0x7FFFFFFE → bound 16 → 4 px.
#[test]
fn row26_both_dims_oversized() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 26);
    let w = 0x7FFF_FFFEi32;
    let iters = expected_iterations(w, w);
    assert_eq!(iters, 4, "row26 expected wrapped trip count");
    for _ in 0..500 {
        let data = random_bytes(&mut rng, 16);
        let out = diff_run(&pair, w, w, &data, "row26");
        assert_eq!(&out[16..], &data[16..], "row26 wrote past the wrapped end");
    }
}

/// Row 27 — guard-region integrity with generous padding.
#[test]
fn row27_guard_regions() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..500 {
        let data = random_bytes(&mut rng, 5 * 3);
        // 256-byte poison guards on both sides; diff_run_ex asserts they survive.
        diff_run_ex(&pair, 5, 3, &data, 256, 0, "row27");
    }
}

/// Row 28 — randomized shape fuzz across the trip-count and value axes at once.
#[test]
fn row28_shape_fuzz() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 28);
    // Live capacity large enough for any in-range shape we generate.
    for _ in 0..20_000 {
        let w = rng.range_i32(-9, 9);
        let h = rng.range_i32(-9, 9);
        let iters = expected_iterations(w, h) as usize;
        // Keep the allocation bounded; the generated range maxes out at 81 px.
        assert!(iters <= 81, "unexpected trip count for {w}x{h}: {iters}");
        let n = (iters + 4).max(8);
        let data = random_bytes(&mut rng, n);
        let out = diff_run(&pair, w, h, &data, "row28-fuzz");
        // Bytes past the computed end must be untouched by both.
        let span = iters * PIXEL_SIZE;
        assert_eq!(
            &out[span..],
            &data[span..],
            "row28 wrote past end for w={w} h={h}"
        );
    }
}

/// Extra fuzz over boundary dimension values combined with random data.
#[test]
fn row28b_boundary_dim_fuzz() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0x28B);
    const BOUNDARY: &[i32] = &[
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0001,
        -0x4000_0000,
        -0x3FFF_FFFF,
        -0x2000_0001,
        -0x2000_0000,
        -0x1FFF_FFFF,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        0x1FFF_FFFF,
        0x2000_0000,
        0x2000_0001,
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &w in BOUNDARY {
        for &h in BOUNDARY {
            let iters = expected_iterations(w, h) as usize;
            // Skip combinations that would require a huge live buffer; those are
            // covered by row25 with an explicitly sized arena.
            if iters > 4096 {
                continue;
            }
            let n = (iters + 4).max(8);
            let data = random_bytes(&mut rng, n);
            let out = diff_run(&pair, w, h, &data, "row28b");
            let span = iters * PIXEL_SIZE;
            assert_eq!(
                &out[span..],
                &data[span..],
                "row28b wrote past end for w={w} h={h}"
            );
        }
    }
}
