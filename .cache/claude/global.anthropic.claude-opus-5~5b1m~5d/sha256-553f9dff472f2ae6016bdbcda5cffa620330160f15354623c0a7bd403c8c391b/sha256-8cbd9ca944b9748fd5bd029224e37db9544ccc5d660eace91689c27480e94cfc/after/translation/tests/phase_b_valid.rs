//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their exported `flip_horizontal` symbol and compares the
//! resulting memory byte-for-byte (payload *and* the canary padding around it),
//! plus the post-call contents of the `cp_image_t` struct.

mod common;

use common::*;
use std::ffi::c_int;

/// How many randomized payloads to try for each fixed shape.
const REPS: usize = 32;

/// Drive one fixed `(w, h)` shape with many random payloads.
#[track_caller]
fn sweep_shape(label: &str, w: c_int, h: c_int, reps: usize, seed_extra: u64) {
    let mut rng = Rng::new(SEED ^ seed_extra ^ ((w as u64) << 32) ^ (h as u64));
    for rep in 0..reps {
        let arena = random_arena(&mut rng, w, h);

        // Independent reference model, to be sure the agreed-upon result is the
        // real row swap and not two identically broken implementations.
        let mut expected = arena.clone();
        model_flip(w, h, expected.payload_mut());

        let out = assert_same(&format!("{label} rep{rep}"), w, h, &arena);
        assert_eq!(
            out.bytes, expected.bytes,
            "[{label} rep{rep}] both libs agree but disagree with the reference \
             model of the C loop for w={w} h={h}"
        );
        // Canary padding must be untouched for in-bounds shapes.
        assert!(
            out.bytes[..arena.pix_off].iter().all(|&b| b == 0xC5),
            "[{label} rep{rep}] leading canary clobbered"
        );
        assert!(
            out.bytes[arena.pix_off + arena.payload_len..].iter().all(|&b| b == 0xC5),
            "[{label} rep{rep}] trailing canary clobbered"
        );
        assert_eq!(out.w, w, "[{label}] img.w mutated");
        assert_eq!(out.h, h, "[{label}] img.h mutated");
        assert!(out.pix_unchanged, "[{label}] img.pix mutated");
    }
}

// --- C1..C15: hand-enumerated distinct shapes, each with random payloads ----

#[test]
fn c01_w0_h0_empty() {
    sweep_shape("C1 w=0,h=0", 0, 0, REPS, 1);
}

#[test]
fn c02_w1_h1_single_pixel() {
    sweep_shape("C2 w=1,h=1", 1, 1, REPS, 2);
}

#[test]
fn c03_w1_h2_single_column_even() {
    sweep_shape("C3 w=1,h=2", 1, 2, REPS, 3);
}

#[test]
fn c04_w1_h3_single_column_odd() {
    sweep_shape("C4 w=1,h=3", 1, 3, REPS, 4);
}

#[test]
fn c05_w2_h1_one_row() {
    sweep_shape("C5 w=2,h=1", 2, 1, REPS, 5);
}

#[test]
fn c06_w3_h2_even() {
    sweep_shape("C6 w=3,h=2", 3, 2, REPS, 6);
}

#[test]
fn c07_w4_h5_odd_middle_row() {
    sweep_shape("C7 w=4,h=5", 4, 5, REPS, 7);

    // Explicit check that the middle row (index h/2 == 2) is left alone, which
    // is the behaviour the odd-height C path has.
    let mut rng = Rng::new(SEED ^ 0x707);
    let (w, h) = (4i32, 5i32);
    let arena = random_arena(&mut rng, w, h);
    let out = assert_same("C7 middle-row", w, h, &arena);
    let row = 2usize;
    let start = arena.pix_off + row * w as usize * 4;
    let end = start + w as usize * 4;
    assert_eq!(
        &out.bytes[start..end],
        &arena.bytes[start..end],
        "C7: middle row of an odd-height image must be untouched"
    );
}

#[test]
fn c08_w1_h64_tall_even() {
    sweep_shape("C8 w=1,h=64", 1, 64, REPS, 8);
}

#[test]
fn c09_w1_h65_tall_odd() {
    sweep_shape("C9 w=1,h=65", 1, 65, REPS, 9);
}

#[test]
fn c10_w64_h1_wide_flat() {
    sweep_shape("C10 w=64,h=1", 64, 1, REPS, 10);
}

#[test]
fn c11_w97_h2_wide_even() {
    sweep_shape("C11 w=97,h=2", 97, 2, REPS, 11);
}

#[test]
fn c12_w97_h97_large_odd_square() {
    sweep_shape("C12 w=97,h=97", 97, 97, 8, 12);
}

#[test]
fn c13_w128_h128_large_even_square() {
    sweep_shape("C13 w=128,h=128", 128, 128, 8, 13);
}

#[test]
fn c14_w0_h7_zero_width_positive_flips() {
    // flips = 3 > 0, so the outer loop runs, but `j < 0` is never true:
    // no pixel may be read or written.
    sweep_shape("C14 w=0,h=7", 0, 7, REPS, 14);
}

#[test]
fn c15_w5_h0_zero_height() {
    sweep_shape("C15 w=5,h=0", 5, 0, REPS, 15);
}

// --- C16: full randomized shape sweep --------------------------------------

#[test]
fn c16_random_shape_sweep() {
    let mut rng = Rng::new(SEED ^ 0xA16);
    for case in 0..200 {
        let w = rng.range_i32(1, 40);
        let h = rng.range_i32(1, 40);
        let arena = random_arena(&mut rng, w, h);

        let mut expected = arena.clone();
        model_flip(w, h, expected.payload_mut());

        let out = assert_same(&format!("C16 case{case}"), w, h, &arena);
        assert_eq!(
            out.bytes, expected.bytes,
            "C16 case{case}: w={w} h={h} disagrees with the reference model"
        );
    }
}

// --- C17: pixel payload extremes -------------------------------------------

#[test]
fn c17_payload_extremes() {
    let (w, h) = (7i32, 6i32);
    let len = payload_bytes(w, h);

    let mut patterns: Vec<Vec<u8>> = Vec::new();
    patterns.push(vec![0x00; len]);
    patterns.push(vec![0xFF; len]);
    // Per-channel one-hot: only r, only g, only b, only a set.
    for chan in 0..4usize {
        let mut p = vec![0u8; len];
        for (i, b) in p.iter_mut().enumerate() {
            *b = if i % 4 == chan { 0xFF } else { 0x00 };
        }
        patterns.push(p);
    }
    // Alternating rows of 0x00 / 0xFF: makes a row-index off-by-one visible.
    let mut p = vec![0u8; len];
    for row in 0..h as usize {
        let v = if row % 2 == 0 { 0x00 } else { 0xFF };
        let s = row * w as usize * 4;
        p[s..s + w as usize * 4].fill(v);
    }
    patterns.push(p);
    // Row-index stamp: every byte of row k equals k. Detects any row mixup.
    let mut p = vec![0u8; len];
    for row in 0..h as usize {
        let s = row * w as usize * 4;
        p[s..s + w as usize * 4].fill(row as u8);
    }
    patterns.push(p);

    for (n, pat) in patterns.iter().enumerate() {
        let mut arena = Arena::new(len, 0xC5);
        arena.payload_mut().copy_from_slice(pat);

        let mut expected = arena.clone();
        model_flip(w, h, expected.payload_mut());

        let out = assert_same(&format!("C17 pattern{n}"), w, h, &arena);
        assert_eq!(out.bytes, expected.bytes, "C17 pattern{n} disagrees with the model");
    }
}

// --- C18: coordinate ramp (catches channel swaps / transposition) -----------

#[test]
fn c18_coordinate_ramp() {
    let (w, h) = (17i32, 9i32);
    let mut arena = Arena::new(payload_bytes(w, h), 0xC5);
    {
        let p = arena.payload_mut();
        for y in 0..h as usize {
            for x in 0..w as usize {
                let o = (y * w as usize + x) * 4;
                p[o] = x as u8;
                p[o + 1] = y as u8;
                p[o + 2] = (x ^ y) as u8;
                p[o + 3] = (x + y) as u8;
            }
        }
    }

    let mut expected = arena.clone();
    model_flip(w, h, expected.payload_mut());

    let out = assert_same("C18 ramp", w, h, &arena);
    assert_eq!(out.bytes, expected.bytes, "C18: ramp result disagrees with the model");

    // The `g` channel holds the source row index; after the flip, row y must
    // carry source row h-1-y. This is asserted against the shared output.
    let p = &out.bytes[arena.pix_off..arena.pix_off + arena.payload_len];
    for y in 0..h as usize {
        let expect_src = (h as usize - 1 - y) as u8;
        for x in 0..w as usize {
            let o = (y * w as usize + x) * 4;
            assert_eq!(p[o + 1], expect_src, "C18: row {y} col {x} came from the wrong source row");
            assert_eq!(p[o], x as u8, "C18: column index changed — this is a row flip only");
        }
    }
}

// --- C19: involution -------------------------------------------------------

#[test]
fn c19_double_application_is_identity() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xB19);
    for case in 0..60 {
        let w = rng.range_i32(0, 24);
        let h = rng.range_i32(0, 24);
        let arena = random_arena(&mut rng, w, h);

        let mut outs = Vec::new();
        for lib in [&p.c, &p.rust] {
            let mut a = arena.clone();
            let pix = a.pix_ptr();
            let mut img = CpImage { w, h, pix };
            unsafe {
                lib.flip(&mut img);
                lib.flip(&mut img);
            }
            outs.push(a.bytes);
        }
        assert_eq!(outs[0], outs[1], "C19 case{case}: double flip diverged (w={w} h={h})");
        assert_eq!(
            outs[0], arena.bytes,
            "C19 case{case}: double flip is not the identity (w={w} h={h})"
        );
    }
}

// --- C20: unaligned pix pointer --------------------------------------------

#[test]
fn c20_unaligned_pix() {
    let (w, h) = (6i32, 4i32);
    let len = payload_bytes(w, h);
    let mut rng = Rng::new(SEED ^ 0xC20);
    for shift in 0..8usize {
        let mut arena = Arena::with_pad(len, 0xC5, PAD, shift);
        rng.fill(arena.payload_mut());

        let mut expected = arena.clone();
        model_flip(w, h, expected.payload_mut());

        let out = assert_same(&format!("C20 shift{shift}"), w, h, &arena);
        assert_eq!(out.bytes, expected.bytes, "C20 shift{shift} disagrees with the model");
    }
}

// --- C21: the struct itself is never mutated -------------------------------

#[test]
fn c21_struct_preserved() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xD21);
    for case in 0..80 {
        let w = rng.range_i32(0, 20);
        let h = rng.range_i32(0, 20);
        let arena = random_arena(&mut rng, w, h);

        for lib in [&p.c, &p.rust] {
            let mut a = arena.clone();
            let pix = a.pix_ptr();
            let mut img = CpImage { w, h, pix };
            let before = unsafe {
                std::slice::from_raw_parts((&raw const img).cast::<u8>(), size_of::<CpImage>())
                    .to_vec()
            };
            unsafe { lib.flip(&mut img) };
            let after = unsafe {
                std::slice::from_raw_parts((&raw const img).cast::<u8>(), size_of::<CpImage>())
                    .to_vec()
            };
            assert_eq!(
                before, after,
                "C21 case{case}: {} mutated the cp_image_t struct (w={w} h={h})",
                lib.name
            );
        }
    }
}

// --- C22: repeated application, parity -------------------------------------

#[test]
fn c22_repeated_application_parity() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE22);
    for case in 0..20 {
        let w = rng.range_i32(1, 12);
        let h = rng.range_i32(1, 12);
        let arena = random_arena(&mut rng, w, h);

        let mut a_c = arena.clone();
        let mut a_r = arena.clone();
        let pix_c = a_c.pix_ptr();
        let pix_r = a_r.pix_ptr();
        let mut img_c = CpImage { w, h, pix: pix_c };
        let mut img_r = CpImage { w, h, pix: pix_r };

        for n in 1..=10u32 {
            unsafe {
                p.c.flip(&mut img_c);
                p.rust.flip(&mut img_r);
            }
            assert_eq!(
                a_c.bytes, a_r.bytes,
                "C22 case{case}: diverged after {n} applications (w={w} h={h})"
            );
            if n % 2 == 0 {
                assert_eq!(
                    a_c.bytes, arena.bytes,
                    "C22 case{case}: even application count must restore the original"
                );
            }
        }
    }
}

// --- C23: no writes outside [pix, pix + w*h) -------------------------------

#[test]
fn c23_no_out_of_bounds_writes() {
    let mut rng = Rng::new(SEED ^ 0xF23);
    for &(w, h) in &[(2i32, 3i32), (1, 2), (3, 4), (5, 5), (8, 2)] {
        for canary in [0x00u8, 0xFF, 0xA5] {
            let mut arena = Arena::new(payload_bytes(w, h), canary);
            rng.fill(arena.payload_mut());

            let out = assert_same(&format!("C23 w={w},h={h},canary={canary:#x}"), w, h, &arena);
            assert!(
                out.bytes[..arena.pix_off].iter().all(|&b| b == canary),
                "C23 w={w} h={h}: leading canary written"
            );
            assert!(
                out.bytes[arena.pix_off + arena.payload_len..].iter().all(|&b| b == canary),
                "C23 w={w} h={h}: trailing canary written"
            );
        }
    }
}

// --- layout sanity ---------------------------------------------------------

#[test]
fn layout_c_struct_layout_assumptions() {
    assert_eq!(size_of::<CpPixel>(), 4);
    assert_eq!(align_of::<CpPixel>(), 1);
    assert_eq!(size_of::<c_int>(), 4);
    assert_eq!(size_of::<CpImage>(), 16);
    assert_eq!(align_of::<CpImage>(), align_of::<*mut u8>());
    let img = CpImage { w: 1, h: 2, pix: std::ptr::null_mut() };
    let base = (&raw const img) as usize;
    assert_eq!((&raw const img.w) as usize - base, 0);
    assert_eq!((&raw const img.h) as usize - base, 4);
    assert_eq!((&raw const img.pix) as usize - base, 8);
}
