//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH shared objects
//! through their exported `premultiply` symbol and compares the resulting
//! pixel buffers byte-for-byte.

mod support;

use support::{diff_bytes, diff_bytes_with_slack, load_pair, Lib, Rng};

/// Allocate a 4-byte-aligned byte buffer of `pixels * 4` bytes.
fn buf(pixels: usize) -> Vec<u8> {
    vec![0u8; pixels * 4]
}

// --------------------------------------------------------------------------
// Row 1 — exhaustive value domain, r == g == b.
// --------------------------------------------------------------------------
#[test]
fn row01_exhaustive_channel_alpha_pairs() {
    let (c, r) = load_pair();
    // 256 * 256 pixels: every (channel, alpha) pair exactly once.
    let mut input = buf(256 * 256);
    for a in 0usize..256 {
        for v in 0usize..256 {
            let i = (a * 256 + v) * 4;
            input[i] = v as u8;
            input[i + 1] = v as u8;
            input[i + 2] = v as u8;
            input[i + 3] = a as u8;
        }
    }
    let out = diff_bytes(&c, &r, "row01", 256, 256, &input);

    // Independent sanity anchors on the C semantics (not a substitute for the
    // differential check, just a guard against both libs being wrong in the
    // same trivial way, e.g. doing nothing at all).
    // a == 255 must preserve the channel exactly.
    let i = (255 * 256 + 200) * 4;
    assert_eq!(out[i], 200, "alpha=255 must preserve the channel");
    // a == 0 must zero the channel.
    let i = (0 * 256 + 200) * 4;
    assert_eq!(out[i], 0, "alpha=0 must zero the channel");
    // alpha itself is never written.
    for a in 0usize..256 {
        let i = (a * 256 + 7) * 4 + 3;
        assert_eq!(out[i], a as u8, "alpha byte must be left untouched");
    }
}

// --------------------------------------------------------------------------
// Row 2 — exhaustive sweep with three distinct channel values per pixel.
// --------------------------------------------------------------------------
#[test]
fn row02_exhaustive_distinct_channels() {
    let (c, r) = load_pair();
    let mut input = buf(256 * 256);
    for a in 0usize..256 {
        for v in 0usize..256 {
            let i = (a * 256 + v) * 4;
            input[i] = v as u8;
            input[i + 1] = (v.wrapping_add(85) & 0xFF) as u8;
            input[i + 2] = (v.wrapping_add(170) & 0xFF) as u8;
            input[i + 3] = a as u8;
        }
    }
    diff_bytes(&c, &r, "row02", 256, 256, &input);
}

// --------------------------------------------------------------------------
// Row 3 — w=2, h=1.
// Row 4 — w=1, h=2.
// Row 23 — w=1, h=1.
// --------------------------------------------------------------------------
fn fixed_shape_fuzz(c: &Lib, r: &Lib, label: &str, w: i32, h: i32, seed: u64, iters: usize) {
    let mut rng = Rng::new(seed);
    let px = (w as usize) * (h as usize);
    for it in 0..iters {
        let mut input = buf(px);
        rng.fill_bytes(&mut input);
        diff_bytes(c, r, &format!("{label}/iter{it}/seed{seed}"), w, h, &input);
    }
}

#[test]
fn row23_single_pixel_random() {
    let (c, r) = load_pair();
    fixed_shape_fuzz(&c, &r, "row23", 1, 1, 0x1111_1111, 20_000);
}

#[test]
fn row03_two_pixels_one_row() {
    let (c, r) = load_pair();
    fixed_shape_fuzz(&c, &r, "row03", 2, 1, 0x3333_3333, 20_000);
}

#[test]
fn row04_two_pixels_two_rows() {
    let (c, r) = load_pair();
    fixed_shape_fuzz(&c, &r, "row04", 1, 2, 0x4444_4444, 20_000);
}

// --------------------------------------------------------------------------
// Row 5 — single column, w=1, h=N.
// --------------------------------------------------------------------------
#[test]
fn row05_single_column_random_height() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x5555_5555);
    for it in 0..300 {
        let h = rng.range(1, 4096) as i32;
        let mut input = buf(h as usize);
        rng.fill_bytes(&mut input);
        diff_bytes(&c, &r, &format!("row05/iter{it}"), 1, h, &input);
    }
}

// --------------------------------------------------------------------------
// Row 6 — single row, w=N, h=1.
// --------------------------------------------------------------------------
#[test]
fn row06_single_row_random_width() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x6666_6666);
    for it in 0..300 {
        let w = rng.range(1, 4096) as i32;
        let mut input = buf(w as usize);
        rng.fill_bytes(&mut input);
        diff_bytes(&c, &r, &format!("row06/iter{it}"), w, 1, &input);
    }
}

// --------------------------------------------------------------------------
// Row 7 — general 2-D geometry.
// --------------------------------------------------------------------------
#[test]
fn row07_general_2d_random() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x7777_7777);
    for it in 0..2_000 {
        let w = rng.range(1, 64) as i32;
        let h = rng.range(1, 64) as i32;
        let mut input = buf((w * h) as usize);
        rng.fill_bytes(&mut input);
        diff_bytes(&c, &r, &format!("row07/iter{it}"), w, h, &input);
    }
}

// --------------------------------------------------------------------------
// Row 8 — equal-product geometries must agree with each other in BOTH libs.
// --------------------------------------------------------------------------
#[test]
fn row08_equal_product_geometries() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x8888_8888);
    // factor pairs of 24, 36, 48, 60, 64 and a few primes
    let products: [u32; 7] = [1, 2, 24, 36, 48, 60, 64];
    for &n in products.iter() {
        for it in 0..200 {
            let mut input = buf(n as usize);
            rng.fill_bytes(&mut input);
            let mut reference: Option<Vec<u8>> = None;
            for w in 1..=n {
                if n % w != 0 {
                    continue;
                }
                let h = n / w;
                let out = diff_bytes(
                    &c,
                    &r,
                    &format!("row08/n{n}/{w}x{h}/iter{it}"),
                    w as i32,
                    h as i32,
                    &input,
                );
                match &reference {
                    None => reference = Some(out),
                    Some(prev) => assert_eq!(
                        prev, &out,
                        "row08: {w}x{h} disagrees with an equal-product geometry (n={n})"
                    ),
                }
            }
        }
    }
}

// --------------------------------------------------------------------------
// Rows 9–12 — forced alpha values.
// --------------------------------------------------------------------------
fn forced_alpha(c: &Lib, r: &Lib, label: &str, alpha: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for it in 0..400 {
        let w = rng.range(1, 32) as i32;
        let h = rng.range(1, 32) as i32;
        let mut input = buf((w * h) as usize);
        rng.fill_bytes(&mut input);
        for px in input.chunks_mut(4) {
            px[3] = alpha;
        }
        let out = diff_bytes(c, r, &format!("{label}/a{alpha}/iter{it}"), w, h, &input);
        if alpha == 0 {
            for (k, px) in out.chunks(4).enumerate() {
                assert_eq!(
                    (px[0], px[1], px[2]),
                    (0, 0, 0),
                    "{label}: alpha=0 must zero RGB (pixel {k})"
                );
                assert_eq!(px[3], 0, "{label}: alpha must be preserved");
            }
        }
        if alpha == 255 {
            for (k, (o, i)) in out.chunks(4).zip(input.chunks(4)).enumerate() {
                assert_eq!(
                    o, i,
                    "{label}: alpha=255 must be an exact identity (pixel {k})"
                );
            }
        }
    }
}

#[test]
fn row09_alpha_zero() {
    let (c, r) = load_pair();
    forced_alpha(&c, &r, "row09", 0, 0x9999_9999);
}

#[test]
fn row10_alpha_full() {
    let (c, r) = load_pair();
    forced_alpha(&c, &r, "row10", 255, 0xAAAA_AAAA);
}

#[test]
fn row11_alpha_one() {
    let (c, r) = load_pair();
    forced_alpha(&c, &r, "row11", 1, 0xBBBB_BBBB);
}

#[test]
fn row12_alpha_near_half_and_max() {
    let (c, r) = load_pair();
    for (i, a) in [127u8, 128, 129, 254, 2].iter().enumerate() {
        forced_alpha(&c, &r, "row12", *a, 0xCCCC_0000 + i as u64);
    }
}

// --------------------------------------------------------------------------
// Rows 13–14 — forced RGB extremes with random alpha.
// --------------------------------------------------------------------------
fn forced_rgb(c: &Lib, r: &Lib, label: &str, v: u8, seed: u64) {
    let mut rng = Rng::new(seed);
    for it in 0..400 {
        let w = rng.range(1, 32) as i32;
        let h = rng.range(1, 32) as i32;
        let mut input = buf((w * h) as usize);
        rng.fill_bytes(&mut input);
        for px in input.chunks_mut(4) {
            px[0] = v;
            px[1] = v;
            px[2] = v;
        }
        diff_bytes(c, r, &format!("{label}/v{v}/iter{it}"), w, h, &input);
    }
}

#[test]
fn row13_rgb_zero() {
    let (c, r) = load_pair();
    forced_rgb(&c, &r, "row13", 0x00, 0xDDDD_DDDD);
}

#[test]
fn row14_rgb_max() {
    let (c, r) = load_pair();
    forced_rgb(&c, &r, "row14", 0xFF, 0xEEEE_EEEE);
}

// --------------------------------------------------------------------------
// Row 15 — misaligned pixel pointer (+1, +2, +3 bytes).
// The C accesses through `uint8_t *`, so this is legal and must match.
// --------------------------------------------------------------------------
#[test]
fn row15_misaligned_buffer() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x0F0F_0F0F);
    for offset in 1usize..=3 {
        for it in 0..300 {
            let w = rng.range(1, 32) as i32;
            let h = rng.range(1, 32) as i32;
            let px = (w * h) as usize;
            let total = px * 4 + 4; // room for the shift
            let mut seed_bytes = vec![0u8; total];
            rng.fill_bytes(&mut seed_bytes);

            let mut cb = seed_bytes.clone();
            let mut rb = seed_bytes.clone();
            unsafe {
                c.call_raw(w, h, cb.as_mut_ptr().add(offset) as *mut support::CPixel);
                r.call_raw(w, h, rb.as_mut_ptr().add(offset) as *mut support::CPixel);
            }
            assert_eq!(
                cb, rb,
                "row15: divergence at misalignment +{offset} (w={w} h={h} iter={it})"
            );
            // Bytes before the shifted start must be untouched by both.
            assert_eq!(&cb[..offset], &seed_bytes[..offset], "row15: underrun");
        }
    }
}

// --------------------------------------------------------------------------
// Row 16 — over-allocated buffer: the slack must be untouched by both.
// --------------------------------------------------------------------------
#[test]
fn row16_over_allocated_slack_untouched() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x1234_5678);
    for it in 0..500 {
        let w = rng.range(1, 32) as i32;
        let h = rng.range(1, 32) as i32;
        let live = (w * h) as usize * 4;
        let mut input = buf((w * h) as usize + 64);
        rng.fill_bytes(&mut input);
        diff_bytes_with_slack(&c, &r, &format!("row16/iter{it}"), w, h, &input, live);
    }
}

// --------------------------------------------------------------------------
// Row 17 — repeated application: state/ordering differences.
// --------------------------------------------------------------------------
#[test]
fn row17_repeat_application() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x2468_ACE0);
    for it in 0..500 {
        let w = rng.range(1, 32) as i32;
        let h = rng.range(1, 32) as i32;
        let mut cb = buf((w * h) as usize);
        rng.fill_bytes(&mut cb);
        let mut rb = cb.clone();
        for pass in 0..4 {
            c.call_bytes(w, h, &mut cb);
            r.call_bytes(w, h, &mut rb);
            assert_eq!(
                cb, rb,
                "row17: divergence after pass {pass} (w={w} h={h} iter={it})"
            );
        }
    }
}

// --------------------------------------------------------------------------
// Row 18 — stride*h wraps to a small POSITIVE bound (+12): exactly 3 pixels.
// --------------------------------------------------------------------------
#[test]
fn row18_bound_wraps_to_small_positive() {
    let (c, r) = load_pair();
    let w = 3i32;
    let h = 0x4000_0001i32; // 2^30 + 1
    assert_eq!(
        support::c_loop_bound(w, h),
        12,
        "precondition: the wrapped bound is +12"
    );
    let mut rng = Rng::new(0x1818_1818);
    for it in 0..2_000 {
        // Give the walk far more room than 12 bytes so an over-walk would show.
        let mut input = buf(64);
        rng.fill_bytes(&mut input);
        diff_bytes_with_slack(&c, &r, &format!("row18/iter{it}"), w, h, &input, 12);
    }
}

// --------------------------------------------------------------------------
// Row 19 — w<0 and h<0: bound wraps POSITIVE, so pixels ARE processed.
// --------------------------------------------------------------------------
#[test]
fn row19_both_dimensions_negative_processes_pixels() {
    let (c, r) = load_pair();
    let (w, h) = (-2i32, -3i32);
    assert_eq!(
        support::c_loop_bound(w, h),
        24,
        "precondition: (-2)*4*(-3) == +24"
    );
    let mut rng = Rng::new(0x1919_1919);
    for it in 0..2_000 {
        let mut input = buf(32);
        rng.fill_bytes(&mut input);
        let out = diff_bytes_with_slack(&c, &r, &format!("row19/iter{it}"), w, h, &input, 24);
        // Guard against "both did nothing": at least sometimes the first pixel
        // must change. Deterministically check the identity property instead:
        // alpha is preserved and RGB <= input RGB.
        for k in 0..6 {
            assert_eq!(out[k * 4 + 3], input[k * 4 + 3], "row19: alpha changed");
            for ch in 0..3 {
                assert!(
                    out[k * 4 + ch] <= input[k * 4 + ch],
                    "row19: premultiplied channel grew"
                );
            }
        }
    }
}

// --------------------------------------------------------------------------
// Row 20 — negative x negative sweep.
// --------------------------------------------------------------------------
#[test]
fn row20_negative_negative_sweep() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x2020_2020);
    for w in -8i32..=-1 {
        for h in -8i32..=-1 {
            let touched = support::c_touched_bytes(w, h);
            let cap = touched + 64;
            for it in 0..50 {
                let mut input = buf(cap / 4 + 1);
                rng.fill_bytes(&mut input);
                diff_bytes_with_slack(
                    &c,
                    &r,
                    &format!("row20/{w}x{h}/iter{it}"),
                    w,
                    h,
                    &input,
                    touched,
                );
            }
        }
    }
}

// --------------------------------------------------------------------------
// Row 21 — fully arbitrary i32 geometry fuzz.
// --------------------------------------------------------------------------
#[test]
fn row21_arbitrary_i32_geometry_fuzz() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x2121_2121);
    // Safety cap: skip geometries that would require a huge live buffer.
    const CAP: usize = 1 << 16;
    let mut exercised_nonempty = 0usize;
    let mut exercised_empty = 0usize;
    let mut attempts = 0usize;
    while attempts < 200_000 && (exercised_nonempty < 2_000 || exercised_empty < 2_000) {
        attempts += 1;
        // Mix of fully random i32s and "interesting" small/boundary values.
        let pick = |rng: &mut Rng| -> i32 {
            match rng.range(0, 4) {
                0 => rng.next_i32(),
                1 => rng.range(0, 8) as i32,
                2 => -(rng.range(0, 8) as i32),
                3 => {
                    const HOT: [i32; 12] = [
                        0,
                        1,
                        -1,
                        i32::MAX,
                        i32::MIN,
                        0x2000_0000,
                        0x2000_0001,
                        0x4000_0000,
                        0x1000_0000,
                        -0x2000_0000,
                        2,
                        3,
                    ];
                    HOT[rng.range(0, 11) as usize]
                }
                _ => rng.next_i32() >> (rng.range(0, 24) as i32),
            }
        };
        let w = pick(&mut rng);
        let h = pick(&mut rng);
        let touched = support::c_touched_bytes(w, h);
        if touched > CAP {
            continue;
        }
        if touched == 0 {
            if exercised_empty >= 4_000 {
                continue;
            }
            exercised_empty += 1;
        } else {
            if exercised_nonempty >= 4_000 {
                continue;
            }
            exercised_nonempty += 1;
        }
        let pixels = touched / 4 + 16;
        let mut input = buf(pixels);
        rng.fill_bytes(&mut input);
        diff_bytes_with_slack(
            &c,
            &r,
            &format!("row21/w{w}/h{h}"),
            w,
            h,
            &input,
            touched,
        );
    }
    assert!(
        exercised_nonempty >= 1_000,
        "row21: too few non-empty geometries exercised ({exercised_nonempty})"
    );
    assert!(
        exercised_empty >= 1_000,
        "row21: too few no-op geometries exercised ({exercised_empty})"
    );
}

// --------------------------------------------------------------------------
// Row 22 — boundary-biased byte distribution.
// --------------------------------------------------------------------------
#[test]
fn row22_boundary_biased_bytes() {
    let (c, r) = load_pair();
    let mut rng = Rng::new(0x2222_2222);
    for it in 0..3_000 {
        let w = rng.range(1, 48) as i32;
        let h = rng.range(1, 48) as i32;
        let mut input = buf((w * h) as usize);
        rng.fill_boundary(&mut input);
        diff_bytes(&c, &r, &format!("row22/iter{it}"), w, h, &input);
    }
}
