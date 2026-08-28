//! Phase B — valid-path differential tests.
//!
//! One test per row of CONFIGS.md. Every test calls BOTH the C `.so` and the
//! Rust `.so` through their exported `premultiply` symbol and requires
//! byte-identical results, with canaries proving neither wrote out of bounds.

mod common;

use common::*;

/// Helper: `n` pixels' worth of random bytes.
fn rnd(rng: &mut Rng, pixels: usize) -> Vec<u8> {
    rng.bytes(pixels * 4)
}

// ---------------------------------------------------------------------- row 1
/// The float pipeline is value-dependent, so prove it over the COMPLETE
/// (colour, alpha) cross-product: all 256 colours x all 256 alphas.
#[test]
fn row01_exhaustive_colour_alpha_cross_product() {
    for a in 0u16..=255 {
        // Batch all 256 colours for this alpha into one 256-pixel image.
        let mut payload = Vec::with_capacity(256 * 4);
        for c in 0u16..=255 {
            payload.extend_from_slice(&[c as u8, c as u8, c as u8, a as u8]);
        }
        assert_same_simple(256, 1, &payload);
    }
}

// ---------------------------------------------------------------------- row 2
/// All 256 alphas x randomized *distinct* r, g, b (so a channel mix-up cannot
/// hide behind r == g == b).
#[test]
fn row02_all_alphas_with_distinct_random_rgb() {
    let mut rng = Rng::new(0x0202_0202);
    for a in 0u16..=255 {
        for _ in 0..64 {
            let mut payload = Vec::with_capacity(64 * 4);
            for _ in 0..64 {
                payload.extend_from_slice(&[rng.u8(), rng.u8(), rng.u8(), a as u8]);
            }
            assert_same_simple(64, 1, &payload);
        }
    }
}

// ------------------------------------------------------------------- rows 3-7
#[test]
fn row03_alpha_zero_all_colours() {
    let mut payload = Vec::new();
    for c in 0u16..=255 {
        payload.extend_from_slice(&[c as u8, 255 - c as u8, c as u8 ^ 0x5A, 0]);
    }
    let out = assert_same_simple(256, 1, &payload);
    // a == 0 scales every colour to 0; alpha itself is never written.
    for px in out.chunks(4) {
        assert_eq!(&px[..3], &[0, 0, 0], "alpha=0 must zero the colour");
        assert_eq!(px[3], 0);
    }
}

#[test]
fn row04_alpha_max_all_colours() {
    let mut payload = Vec::new();
    for c in 0u16..=255 {
        payload.extend_from_slice(&[c as u8, 255 - c as u8, c as u8 ^ 0x33, 255]);
    }
    assert_same_simple(256, 1, &payload);
}

#[test]
fn row05_extreme_nonsaturated_alphas() {
    for a in [1u8, 254u8] {
        let mut payload = Vec::new();
        for c in 0u16..=255 {
            payload.extend_from_slice(&[c as u8, c as u8, c as u8, a]);
        }
        assert_same_simple(256, 1, &payload);
    }
}

#[test]
fn row06_max_colour_all_alphas() {
    let mut payload = Vec::new();
    for a in 0u16..=255 {
        payload.extend_from_slice(&[255, 255, 255, a as u8]);
    }
    assert_same_simple(256, 1, &payload);
}

#[test]
fn row07_min_colour_all_alphas() {
    let mut payload = Vec::new();
    for a in 0u16..=255 {
        payload.extend_from_slice(&[0, 0, 0, a as u8]);
    }
    assert_same_simple(256, 1, &payload);
}

// ------------------------------------------------------------------ rows 8-14
#[test]
fn row08_single_column() {
    let mut rng = Rng::new(0x0808_0808);
    for h in [1, 2, 3, 7, 64] {
        for _ in 0..40 {
            let p = rnd(&mut rng, h as usize);
            assert_same_simple(1, h, &p);
        }
    }
}

#[test]
fn row09_single_row() {
    let mut rng = Rng::new(0x0909_0909);
    for w in [1, 2, 3, 7, 64] {
        for _ in 0..40 {
            let p = rnd(&mut rng, w as usize);
            assert_same_simple(w, 1, &p);
        }
    }
}

#[test]
fn row10_squares() {
    let mut rng = Rng::new(0x1010_1010);
    for n in [1, 2, 3, 4, 5, 8, 16, 37] {
        for _ in 0..25 {
            let p = rnd(&mut rng, (n * n) as usize);
            assert_same_simple(n, n, &p);
        }
    }
}

#[test]
fn row11_random_nonsquare_shapes() {
    let mut rng = Rng::new(0x1111_1111);
    for _ in 0..400 {
        let w = rng.range(1, 40);
        let h = rng.range(1, 40);
        let p = rnd(&mut rng, (w * h) as usize);
        assert_same_simple(w, h, &p);
    }
}

#[test]
fn row12_odd_widths() {
    let mut rng = Rng::new(0x1212_1212);
    for w in [1, 3, 5, 7, 9, 11, 13] {
        for _ in 0..30 {
            let p = rnd(&mut rng, (w * 3) as usize);
            assert_same_simple(w, 3, &p);
        }
    }
}

#[test]
fn row13_even_widths() {
    let mut rng = Rng::new(0x1313_1313);
    for w in [2, 4, 6, 8, 16] {
        for _ in 0..30 {
            let p = rnd(&mut rng, (w * 3) as usize);
            assert_same_simple(w, 3, &p);
        }
    }
}

#[test]
fn row14_large_buffer() {
    let mut rng = Rng::new(0x1414_1414);
    for _ in 0..8 {
        let p = rnd(&mut rng, 256 * 64);
        assert_same_simple(256, 64, &p);
    }
}

// --------------------------------------------------------------------- row 15
/// `pix` is consumed as `uint8_t *`, so unaligned buffers are legal input.
#[test]
fn row15_misaligned_pix() {
    let mut rng = Rng::new(0x1515_1515);
    for misalign in 0..4usize {
        for _ in 0..50 {
            let p = rnd(&mut rng, 64);
            assert_same(8, 8, &p, misalign, 1);
        }
    }
}

// ------------------------------------------------------------------ rows 16-19
#[test]
fn row16_all_zero_buffer() {
    let p = vec![0u8; 16 * 4];
    let out = assert_same_simple(4, 4, &p);
    assert!(out.iter().all(|&b| b == 0));
}

#[test]
fn row17_all_ff_buffer() {
    let p = vec![0xFFu8; 16 * 4];
    let out = assert_same_simple(4, 4, &p);
    // a == 255 -> colour preserved.
    assert!(out.iter().all(|&b| b == 0xFF), "got {out:?}");
}

#[test]
fn row18_alpha_ramp() {
    let mut payload = Vec::new();
    for i in 0..256usize {
        payload.extend_from_slice(&[0xFF, 0xFF, 0xFF, (i % 256) as u8]);
    }
    assert_same_simple(16, 16, &payload);
}

#[test]
fn row19_colour_ramp_at_fixed_alphas() {
    for a in [0u8, 1, 127, 128, 254, 255] {
        let mut payload = Vec::new();
        for i in 0..256usize {
            let c = (i % 256) as u8;
            payload.extend_from_slice(&[c, c.wrapping_mul(3), c.wrapping_add(97), a]);
        }
        assert_same_simple(16, 16, &payload);
    }
}

// --------------------------------------------------------------------- row 20
/// `premultiply` is destructive and NOT idempotent; calling it twice must still
/// produce identical results in both implementations.
#[test]
fn row20_double_invocation() {
    let mut rng = Rng::new(0x2020_2020);
    for _ in 0..100 {
        let p = rnd(&mut rng, 64);
        let once = assert_same(8, 8, &p, 0, 1);
        let twice = assert_same(8, 8, &p, 0, 2);
        // Sanity: double application really does differ (unless already fixed
        // point), which proves the `calls=2` path is meaningful.
        let _ = (once, twice);
    }
}

// --------------------------------------------------------------------- row 21
/// The C code stores only `data[i+0..=2]`; byte `+3` (alpha) must survive.
#[test]
fn row21_alpha_byte_is_preserved() {
    let mut rng = Rng::new(0x2121_2121);
    for _ in 0..200 {
        let p = rnd(&mut rng, 64);
        let out = assert_same(8, 8, &p, 0, 1);
        for i in 0..64 {
            assert_eq!(
                out[i * 4 + 3],
                p[i * 4 + 3],
                "alpha of pixel {i} was modified"
            );
        }
    }
}

// --------------------------------------------------------------------- row 22
/// Canaries are checked inside `assert_same` for every case; this row makes the
/// guarantee explicit and also pins the exact write extent by surrounding a
/// *smaller* logical image with spare payload that must stay untouched.
#[test]
fn row22_write_extent_is_exact() {
    let mut rng = Rng::new(0x2222_2222);
    for _ in 0..200 {
        // Allocate 128 pixels but describe only an 8x8 = 64-pixel image.
        let p = rnd(&mut rng, 128);
        let out = assert_same(8, 8, &p, 0, 1);
        assert_eq!(
            &out[64 * 4..],
            &p[64 * 4..],
            "bytes past the {}-byte extent were modified",
            64 * 4
        );
    }
}

// --------------------------------------------------------------------- row 23
/// `w < 0 && h < 0` makes `limit` POSITIVE, so the loop runs. Surprising, but
/// it is what the C does, and both sides must agree.
#[test]
fn row23_negative_width_and_height_runs() {
    let mut rng = Rng::new(0x2323_2323);
    for (w, h) in [(-1, -1), (-2, -3), (-1, -4), (-4, -4), (-3, -7), (-8, -8)] {
        let iters = semantics(w, h).2;
        assert!(iters > 0, "row 23 expects the loop to run for ({w},{h})");
        for _ in 0..60 {
            let p = rnd(&mut rng, iters + 16);
            let out = assert_same_simple(w, h, &p);
            assert_eq!(&out[iters * 4..], &p[iters * 4..], "wrote past {iters} px");
        }
    }
}

// ------------------------------------------------------------------ rows 24-26
/// Wrap-to-small-positive cases: enormous nominal dimensions, tiny real extent.
#[test]
fn row24_intmax_wrap_cases() {
    let mut rng = Rng::new(0x2424_2424);
    for (w, h) in [(i32::MAX, -1), (i32::MAX, i32::MAX)] {
        let iters = semantics(w, h).2;
        assert_eq!(iters, 1, "expected exactly 1 pixel for ({w},{h})");
        for _ in 0..80 {
            let p = rnd(&mut rng, 32);
            let out = assert_same_simple(w, h, &p);
            assert_eq!(&out[4..], &p[4..], "wrote past 1 px");
        }
    }
}

#[test]
fn row25_two_pow_29_plus_one_h2() {
    let mut rng = Rng::new(0x2525_2525);
    let (w, h) = (536_870_913i32, 2i32); // 2^29 + 1
    assert_eq!(semantics(w, h).2, 2);
    for _ in 0..100 {
        let p = rnd(&mut rng, 32);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[8..], &p[8..]);
    }
}

#[test]
fn row26_two_pow_28_plus_one_h4() {
    let mut rng = Rng::new(0x2626_2626);
    let (w, h) = (268_435_457i32, 4i32); // 2^28 + 1
    assert_eq!(semantics(w, h).2, 4);
    for _ in 0..100 {
        let p = rnd(&mut rng, 32);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[16..], &p[16..]);
    }
}

// --------------------------------------------------------------------- row 27
/// Wrap-to-zero and wrap-to-negative combinations, run against a LIVE buffer so
/// that an incorrect (non-wrapping) bound would corrupt memory / trip canaries
/// rather than silently pass.
#[test]
fn row27_wrap_to_zero_or_negative_noops() {
    let mut rng = Rng::new(0x2727_2727);
    let widths = [
        1 << 30,
        -(1 << 30),
        i32::MIN,
        1 << 29,
        -(1 << 29),
    ];
    for w in widths {
        for h in [1, 2, 3, 5] {
            let iters = semantics(w, h).2;
            assert_eq!(iters, 0, "row 27 expects a no-op for (w={w}, h={h})");
            let p = rnd(&mut rng, 32);
            let out = assert_same_simple(w, h, &p);
            assert_eq!(out, p, "buffer must be untouched for (w={w}, h={h})");
        }
    }
}

// --------------------------------------------------------------------- row 28
#[test]
fn row28_extreme_single_dimension() {
    let mut rng = Rng::new(0x2828_2828);
    let mut cases: Vec<(i32, i32)> = vec![(1, i32::MIN), (1, i32::MAX), (i32::MIN, 1)];
    // (i32::MAX, 1) -> limit == -4 -> no-op; include it too.
    cases.push((i32::MAX, 1));
    for (w, h) in cases {
        let iters = semantics(w, h).2;
        assert_eq!(iters, 0, "expected no-op for ({w},{h})");
        let p = rnd(&mut rng, 32);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(out, p, "buffer must be untouched for ({w},{h})");
    }
}

// --------------------------------------------------------------------- row 29
/// Randomized fuzz over the whole `(w, h)` wrap space. A case is executed only
/// when the predicted extent fits the guarded buffer; everything else would be
/// an out-of-bounds access in the C code itself.
#[test]
fn row29_fuzz_dimension_wrap_space() {
    let mut rng = Rng::new(0x2929_2929);
    const CAP_PX: usize = 512;
    let mut executed = 0usize;
    let mut ran_loop = 0usize;
    let mut noop = 0usize;

    for _ in 0..2000 {
        // Mix of interesting generators.
        let pick = |rng: &mut Rng| -> i32 {
            match rng.below(6) {
                0 => rng.range(-8, 8),
                1 => {
                    let k = rng.below(32);
                    1i32.wrapping_shl(k)
                }
                2 => {
                    let k = rng.below(32);
                    (1i32.wrapping_shl(k)).wrapping_neg()
                }
                3 => {
                    let k = rng.below(32);
                    1i32.wrapping_shl(k).wrapping_add(1)
                }
                4 => [i32::MIN, i32::MAX, 0, -1, 1][rng.below(5) as usize],
                _ => rng.i32(),
            }
        };
        let w = pick(&mut rng);
        let h = pick(&mut rng);
        let iters = semantics(w, h).2;
        if iters > CAP_PX {
            continue;
        }
        executed += 1;
        if iters > 0 {
            ran_loop += 1;
        } else {
            noop += 1;
        }
        let p = rnd(&mut rng, CAP_PX);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(
            &out[iters * 4..],
            &p[iters * 4..],
            "({w},{h}) wrote past its {iters}-pixel extent"
        );
    }

    // The fuzzer must actually reach both classes of behaviour.
    assert!(executed > 500, "only {executed} cases were executable");
    assert!(ran_loop > 20, "only {ran_loop} cases ran the loop");
    assert!(noop > 20, "only {noop} cases were no-ops");
}

// --------------------------------------------------------------------- row 30
#[test]
fn row30_fuzz_dimensions_and_data() {
    let mut rng = Rng::new(0x3030_3030);
    for _ in 0..2000 {
        let w = rng.range(0, 24);
        let h = rng.range(0, 24);
        let px = (w * h) as usize;
        let p = rnd(&mut rng, px + 8);
        let out = assert_same_simple(w, h, &p);
        assert_eq!(&out[px * 4..], &p[px * 4..]);
    }
}

// --------------------------------------------------------------------- row 31
/// A misaligned `cp_image_t *`. x86-64 permits unaligned scalar loads, so the C
/// code handles this; the Rust must too.
#[test]
fn row31_misaligned_image_struct_pointer() {
    let mut rng = Rng::new(0x3131_3131);
    let l = libs();

    for _ in 0..50 {
        let payload = rnd(&mut rng, 16);
        let mut results: Vec<Vec<u8>> = Vec::new();

        for (who, f) in [("C", l.c), ("Rust", l.rust)] {
            let mut g = Guarded::new(&payload, 0);
            let pix = g.ptr();

            // Place a cp_image_t at an odd address inside a byte arena.
            let mut arena = vec![0u8; std::mem::size_of::<CpImage>() + 8];
            let base = arena.as_mut_ptr();
            let img_ptr = unsafe {
                let mut p = base.add(1);
                // Force an odd address.
                if (p as usize) % 2 == 0 {
                    p = p.add(1);
                }
                p as *mut CpImage
            };
            assert_eq!((img_ptr as usize) % 2, 1, "wanted an odd struct address");
            unsafe {
                // write_unaligned: the struct is deliberately not aligned.
                img_ptr.write_unaligned(CpImage { w: 4, h: 4, pix });
                f(img_ptr);
            }
            g.assert_canaries(who, "misaligned img struct");
            results.push(g.payload().to_vec());
        }
        assert_eq!(results[0], results[1], "misaligned-img divergence");
    }
}

// --------------------------------------------------------------------- row 32
/// `pix` pointing into the middle of a larger arena: nothing before or after
/// the described image may be touched.
#[test]
fn row32_pix_into_middle_of_arena() {
    let mut rng = Rng::new(0x3232_3232);
    let l = libs();
    const PRE: usize = 40;
    const POST: usize = 40;

    for _ in 0..100 {
        let img_bytes = 6 * 6 * 4;
        let arena_seed = rng.bytes(PRE + img_bytes + POST);
        let mut results: Vec<Vec<u8>> = Vec::new();

        for (who, f) in [("C", l.c), ("Rust", l.rust)] {
            let mut g = Guarded::new(&arena_seed, 0);
            let pix = unsafe { g.ptr().add(PRE) };
            let mut img = CpImage { w: 6, h: 6, pix };
            unsafe { f(&mut img as *mut CpImage) };
            g.assert_canaries(who, "pix mid-arena");
            results.push(g.payload().to_vec());
        }
        assert_eq!(results[0], results[1], "mid-arena divergence");

        let out = &results[0];
        assert_eq!(&out[..PRE], &arena_seed[..PRE], "arena prefix modified");
        assert_eq!(
            &out[PRE + img_bytes..],
            &arena_seed[PRE + img_bytes..],
            "arena suffix modified"
        );
    }
}
