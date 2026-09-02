//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row runs `REPS` randomized inputs
//! from a fixed seed and compares the FULL backing allocation (payload plus
//! poison guard bands) between the C `.so` and the Rust `.so`.

mod common;

use common::*;

fn rng_for(row: u64) -> Rng {
    Rng::new(SEED ^ (row.wrapping_mul(0x1000_0000_0000_01B3)))
}

// --- row 1 ------------------------------------------------------------------
#[test]
fn cfg_01_empty_0x0() {
    let mut rng = rng_for(1);
    for _ in 0..REPS {
        // A zero-size image still gets a real (guarded) allocation so an
        // out-of-bounds write would be caught.
        assert_same_random("cfg_01", &Case::new(0, 0).alloc(8), &mut rng);
    }
}

// --- row 2 ------------------------------------------------------------------
#[test]
fn cfg_02_h0_random_w() {
    let mut rng = rng_for(2);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        assert_same_random("cfg_02", &Case::new(w, 0).alloc(w as usize), &mut rng);
    }
}

// --- row 3 ------------------------------------------------------------------
#[test]
fn cfg_03_h1_random_w() {
    let mut rng = rng_for(3);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        assert_same_random("cfg_03", &Case::new(w, 1), &mut rng);
    }
}

// --- row 4 ------------------------------------------------------------------
#[test]
fn cfg_04_w0_random_h() {
    let mut rng = rng_for(4);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 32);
        assert_same_random("cfg_04", &Case::new(0, h).alloc(16), &mut rng);
    }
}

// --- row 5 ------------------------------------------------------------------
#[test]
fn cfg_05_min_swap_1x2() {
    let mut rng = rng_for(5);
    for _ in 0..REPS {
        assert_same_random("cfg_05", &Case::new(1, 2), &mut rng);
    }
}

// --- row 6 ------------------------------------------------------------------
#[test]
fn cfg_06_h2_random_w() {
    let mut rng = rng_for(6);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        assert_same_random("cfg_06", &Case::new(w, 2), &mut rng);
    }
}

// --- row 7 ------------------------------------------------------------------
#[test]
fn cfg_07_h3_random_w() {
    let mut rng = rng_for(7);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 64);
        assert_same_random("cfg_07", &Case::new(w, 3), &mut rng);
    }
}

// --- row 8 ------------------------------------------------------------------
#[test]
fn cfg_08_even_h_random_w() {
    let mut rng = rng_for(8);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 16) * 2; // 4..=32, even
        let w = rng.range_i32(1, 64);
        assert_eq!(h % 2, 0);
        assert_same_random("cfg_08", &Case::new(w, h), &mut rng);
    }
}

// --- row 9 ------------------------------------------------------------------
#[test]
fn cfg_09_odd_h_random_w() {
    let mut rng = rng_for(9);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 16) * 2 + 1; // 5..=33, odd
        let w = rng.range_i32(1, 64);
        assert_eq!(h % 2, 1);
        assert_same_random("cfg_09", &Case::new(w, h), &mut rng);
    }
}

// --- row 10 -----------------------------------------------------------------
#[test]
fn cfg_10_tall_thin() {
    let mut rng = rng_for(10);
    for _ in 0..REPS {
        let h = rng.range_i32(2, 256);
        assert_same_random("cfg_10", &Case::new(1, h), &mut rng);
    }
}

// --- row 11 -----------------------------------------------------------------
#[test]
fn cfg_11_wide_flat() {
    let mut rng = rng_for(11);
    for _ in 0..REPS {
        let w = rng.range_i32(512, 4096);
        assert_same_random("cfg_11", &Case::new(w, 2), &mut rng);
    }
}

// --- row 12 -----------------------------------------------------------------
#[test]
fn cfg_12_landscape() {
    let mut rng = rng_for(12);
    for _ in 0..REPS {
        let w = rng.range_i32(17, 200);
        let h = rng.range_i32(2, 16);
        assert_same_random("cfg_12", &Case::new(w, h), &mut rng);
    }
}

// --- row 13 -----------------------------------------------------------------
#[test]
fn cfg_13_portrait() {
    let mut rng = rng_for(13);
    for _ in 0..REPS {
        let w = rng.range_i32(2, 16);
        let h = rng.range_i32(17, 200);
        assert_same_random("cfg_13", &Case::new(w, h), &mut rng);
    }
}

// --- row 14 -----------------------------------------------------------------
#[test]
fn cfg_14_square() {
    let mut rng = rng_for(14);
    for _ in 0..REPS {
        let n = rng.range_i32(2, 48);
        assert_same_random("cfg_14", &Case::new(n, n), &mut rng);
    }
}

// --- row 15 -----------------------------------------------------------------
#[test]
fn cfg_15_large_image() {
    let mut rng = rng_for(15);
    for _ in 0..16 {
        // Keep w*h <= ~120k pixels so the row stays well inside the time budget.
        let w = rng.range_i32(64, 512);
        let h = 120_000 / w.max(1);
        let h = h.clamp(2, 512);
        assert_same_random("cfg_15", &Case::new(w, h), &mut rng);
    }
}

// --- row 16 -----------------------------------------------------------------
#[test]
fn cfg_16_double_application_involution() {
    let mut rng = rng_for(16);
    for _ in 0..REPS {
        let w = rng.range_i32(0, 40);
        let h = rng.range_i32(0, 40);
        let case = Case::new(w, h).calls(2);

        // Both implementations must agree...
        assert_same_random("cfg_16", &case, &mut rng);

        // ...and, per the C's own semantics, two applications restore the
        // original buffer exactly. Verified against the C output so the C
        // remains the ground truth.
        let mut seed = Buf::new(case.alloc_pixels, GUARD_PIXELS, false);
        seed.fill_random(&mut rng);
        let im = impls();
        for f in [im.c, im.rust] {
            let mut work = seed.clone_layout();
            let pix = work.pix_ptr();
            let mut img = cp_image_t { w, h, pix };
            unsafe {
                f(&mut img);
                f(&mut img);
            }
            assert_eq!(
                seed.all_bytes(),
                work.all_bytes(),
                "cfg_16: double application is not an involution for {w}x{h}"
            );
        }
    }
}

// --- row 17 -----------------------------------------------------------------
#[test]
fn cfg_17_interior_pix_with_guards() {
    // `Buf` already places `pix` behind a poison guard band and `assert_same`
    // compares the guards too; this row exercises it across random shapes and
    // additionally asserts the C itself leaves the guards untouched.
    let mut rng = rng_for(17);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 48);
        let h = rng.range_i32(1, 48);
        let case = Case::new(w, h);
        assert_same_random("cfg_17", &case, &mut rng);

        let mut seed = Buf::new(case.alloc_pixels, GUARD_PIXELS, false);
        seed.fill_random(&mut rng);
        let guard_before = seed.all_bytes()[..GUARD_PIXELS * 4].to_vec();
        let im = impls();
        for (name, f) in [("C", im.c), ("Rust", im.rust)] {
            let mut work = seed.clone_layout();
            let pix = work.pix_ptr();
            let mut img = cp_image_t { w, h, pix };
            unsafe { f(&mut img) };
            assert_eq!(
                &work.all_bytes()[..GUARD_PIXELS * 4],
                guard_before.as_slice(),
                "cfg_17: {name} wrote into the leading guard band for {w}x{h}"
            );
        }
    }
}

// --- row 18 -----------------------------------------------------------------
#[test]
fn cfg_18_unaligned_pix() {
    let mut rng = rng_for(18);
    for _ in 0..REPS {
        let w = rng.range_i32(1, 48);
        let h = rng.range_i32(1, 48);
        let case = Case::new(w, h).odd_offset();

        // Sanity: pixels are 4-byte objects, so shifting `pix` by one pixel
        // yields a 4-byte-aligned address that is not 8-byte aligned whenever
        // the allocation base is 8-byte aligned (the normal case on glibc).
        let mut probe = Buf::new(case.alloc_pixels, GUARD_PIXELS, true);
        let addr = probe.pix_ptr() as usize;
        assert_eq!(addr % 4, 0, "pixels are 4-byte objects");

        assert_same_random("cfg_18", &case, &mut rng);
    }
}

// --- row 19 -----------------------------------------------------------------
#[test]
fn cfg_19_degenerate_payloads() {
    let mut rng = rng_for(19);
    let patterns: [(&str, fn(usize) -> cp_pixel_t); 6] = [
        ("all_zero", |_| cp_pixel_t {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }),
        ("all_ff", |_| cp_pixel_t {
            r: 0xFF,
            g: 0xFF,
            b: 0xFF,
            a: 0xFF,
        }),
        ("channel_const", |_| cp_pixel_t {
            r: 0x11,
            g: 0x22,
            b: 0x33,
            a: 0x44,
        }),
        ("index_stamped", |i| cp_pixel_t {
            r: (i & 0xFF) as u8,
            g: ((i >> 8) & 0xFF) as u8,
            b: ((i >> 16) & 0xFF) as u8,
            a: 0x7F,
        }),
        ("alternating", |i| {
            if i % 2 == 0 {
                cp_pixel_t {
                    r: 0xAA,
                    g: 0x55,
                    b: 0xAA,
                    a: 0x55,
                }
            } else {
                cp_pixel_t {
                    r: 0x55,
                    g: 0xAA,
                    b: 0x55,
                    a: 0xAA,
                }
            }
        }),
        ("alpha_only", |i| cp_pixel_t {
            r: 0,
            g: 0,
            b: 0,
            a: (i & 0xFF) as u8,
        }),
    ];

    for (name, pat) in patterns {
        for _ in 0..16 {
            let w = rng.range_i32(1, 40);
            let h = rng.range_i32(1, 40);
            let case = Case::new(w, h);
            let mut seed = Buf::new(case.alloc_pixels, GUARD_PIXELS, false);
            seed.fill_with(pat);
            assert_same(&format!("cfg_19/{name}"), &case, &seed);
        }
    }
}

// --- row 20 -----------------------------------------------------------------
#[test]
fn cfg_20_small_shape_cross_product() {
    let mut rng = rng_for(20);
    for w in 0..=9i32 {
        for h in 0..=9i32 {
            for _ in 0..8 {
                // Always allocate at least a few pixels so a stray write into a
                // "zero-size" image is still detectable.
                let pixels = ((w * h) as usize).max(8);
                let case = Case::new(w, h).alloc(pixels);
                assert_same_random(&format!("cfg_20/{w}x{h}"), &case, &mut rng);
            }
        }
    }
}

// --- row 21 -----------------------------------------------------------------
#[test]
fn cfg_21_struct_reuse_mutating_dims() {
    // Drive the same `cp_image_t` through a sequence of differing dimensions,
    // exactly as a consumer resizing an image would, to prove there is no
    // hidden static state on either side.
    let mut rng = rng_for(21);
    let im = impls();

    for _ in 0..32 {
        let steps: Vec<(i32, i32)> = (0..6)
            .map(|_| (rng.range_i32(0, 24), rng.range_i32(0, 24)))
            .collect();
        let max_pixels = steps
            .iter()
            .map(|&(w, h)| (w * h) as usize)
            .max()
            .unwrap()
            .max(8);

        let mut seed = Buf::new(max_pixels, GUARD_PIXELS, false);
        seed.fill_random(&mut rng);

        let mut outs = Vec::new();
        for f in [im.c, im.rust] {
            let mut work = seed.clone_layout();
            let pix = work.pix_ptr();
            let mut img = cp_image_t { w: 0, h: 0, pix };
            for &(w, h) in &steps {
                img.w = w;
                img.h = h;
                unsafe { f(&mut img) };
            }
            outs.push(work.all_bytes().to_vec());
        }
        assert_eq!(
            outs[0], outs[1],
            "cfg_21: divergence across the mutating-dimension sequence {steps:?}"
        );
    }
}
