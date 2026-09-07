//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the resulting pixel buffers byte-for-byte. Randomized inputs
//! use a fixed seed so failures are reproducible.

mod common;

use common::{bytes, diff, diff_wh, libs, CpImage, CpPixel, Rng};

// --- row 1 -----------------------------------------------------------------
#[test]
fn row01_single_pixel_randomized() {
    let mut rng = Rng::new(0x0000_0001);
    for i in 0..4096 {
        let px = rng.pixels(1);
        diff_wh(&px, 1, 1, &format!("row01 iter {i}"));
    }
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn row02_exhaustive_alpha_x_channel() {
    for a in 0u16..=255 {
        for c in 0u16..=255 {
            let px = [CpPixel {
                r: c as u8,
                g: c as u8,
                b: c as u8,
                a: a as u8,
            }];
            diff_wh(&px, 1, 1, &format!("row02 a={a} c={c}"));
        }
    }
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn row03_channel_ordering_exhaustive_alpha() {
    // Distinct per-channel values catch any r/g/b index swap, and asserting the
    // alpha byte is untouched catches a stray write to data[i+3].
    for a in 0u16..=255 {
        let input = [CpPixel {
            r: 200,
            g: 100,
            b: 37,
            a: a as u8,
        }];
        let out = diff_wh(&input, 1, 1, &format!("row03 a={a}"));
        assert_eq!(out[0].a, a as u8, "row03: alpha must be preserved");
    }
    // And a second, differently ordered triple.
    for a in 0u16..=255 {
        let input = [CpPixel {
            r: 1,
            g: 254,
            b: 128,
            a: a as u8,
        }];
        let out = diff_wh(&input, 1, 1, &format!("row03b a={a}"));
        assert_eq!(out[0].a, a as u8);
    }
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn row04_all_transparent() {
    let mut rng = Rng::new(0x0000_0004);
    for i in 0..64 {
        let mut px = rng.pixels(7 * 5);
        for p in px.iter_mut() {
            p.a = 0;
        }
        let out = diff_wh(&px, 7, 5, &format!("row04 iter {i}"));
        for (n, p) in out.iter().enumerate() {
            assert_eq!(
                (p.r, p.g, p.b, p.a),
                (0, 0, 0, 0),
                "row04: a=0 must zero rgb (pixel {n})"
            );
        }
    }
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn row05_all_opaque_is_identity() {
    let mut rng = Rng::new(0x0000_0005);
    for i in 0..64 {
        let mut px = rng.pixels(7 * 5);
        for p in px.iter_mut() {
            p.a = 255;
        }
        let out = diff_wh(&px, 7, 5, &format!("row05 iter {i}"));
        assert_eq!(
            bytes(&out),
            bytes(&px),
            "row05: a=255 must be a bit-exact identity"
        );
    }
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn row06_near_boundary_alphas() {
    let mut rng = Rng::new(0x0000_0006);
    for a in [1u8, 254u8] {
        for i in 0..128 {
            let mut px = rng.pixels(9 * 3);
            for p in px.iter_mut() {
                p.a = a;
            }
            diff_wh(&px, 9, 3, &format!("row06 a={a} iter {i}"));
        }
    }
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn row07_colours_max() {
    let mut rng = Rng::new(0x0000_0007);
    for i in 0..32 {
        let mut px = rng.pixels(16 * 16);
        for p in px.iter_mut() {
            p.r = 255;
            p.g = 255;
            p.b = 255;
        }
        diff_wh(&px, 16, 16, &format!("row07 iter {i}"));
    }
}

// --- row 8 -----------------------------------------------------------------
#[test]
fn row08_colours_min() {
    let mut rng = Rng::new(0x0000_0008);
    for i in 0..32 {
        let mut px = rng.pixels(16 * 16);
        for p in px.iter_mut() {
            p.r = 0;
            p.g = 0;
            p.b = 0;
        }
        let out = diff_wh(&px, 16, 16, &format!("row08 iter {i}"));
        for p in &out {
            assert_eq!((p.r, p.g, p.b), (0, 0, 0));
        }
    }
}

// --- row 9 -----------------------------------------------------------------
#[test]
fn row09_single_column() {
    let mut rng = Rng::new(0x0000_0009);
    for h in [1usize, 2, 3, 17, 64] {
        for i in 0..32 {
            let px = rng.pixels(h);
            diff_wh(&px, 1, h, &format!("row09 h={h} iter {i}"));
        }
    }
}

// --- row 10 ----------------------------------------------------------------
#[test]
fn row10_single_row() {
    let mut rng = Rng::new(0x0000_000A);
    for w in [1usize, 2, 3, 17, 64] {
        for i in 0..32 {
            let px = rng.pixels(w);
            diff_wh(&px, w, 1, &format!("row10 w={w} iter {i}"));
        }
    }
}

// --- row 11 ----------------------------------------------------------------
#[test]
fn row11_squares() {
    let mut rng = Rng::new(0x0000_000B);
    for n in [2usize, 3, 4, 5, 8, 13, 16, 31, 32] {
        for i in 0..16 {
            let px = rng.pixels(n * n);
            diff_wh(&px, n, n, &format!("row11 n={n} iter {i}"));
        }
    }
}

// --- row 12 ----------------------------------------------------------------
#[test]
fn row12_rectangles() {
    let mut rng = Rng::new(0x0000_000C);
    for (w, h) in [(3usize, 7usize), (7, 3), (1, 255), (255, 1), (17, 19), (64, 3)] {
        for i in 0..16 {
            let px = rng.pixels(w * h);
            diff_wh(&px, w, h, &format!("row12 {w}x{h} iter {i}"));
        }
    }
}

// --- row 13 ----------------------------------------------------------------
#[test]
fn row13_large_image() {
    let mut rng = Rng::new(0x0000_000D);
    for i in 0..8 {
        let px = rng.pixels(131 * 97);
        diff_wh(&px, 131, 97, &format!("row13 iter {i}"));
    }
}

// --- row 14 ----------------------------------------------------------------
#[test]
fn row14_property_random_shape_and_values() {
    let mut rng = Rng::new(0x0000_000E);
    for i in 0..300 {
        let w = rng.range(1, 40) as usize;
        let h = rng.range(1, 40) as usize;
        let px = rng.pixels(w * h);
        diff_wh(&px, w, h, &format!("row14 iter {i} {w}x{h}"));
    }
}

// --- row 15 ----------------------------------------------------------------
#[test]
fn row15_padded_buffer_guard_pixels() {
    // Buffer larger than the logical image: an off-by-one loop bound in either
    // direction shows up as a guard-pixel divergence (too far) or as an
    // unprocessed last pixel (too short, caught by the byte compare).
    let mut rng = Rng::new(0x0000_000F);
    const PAD: usize = 8;
    for (w, h) in [(1usize, 1usize), (3, 3), (5, 4), (16, 16), (13, 7), (40, 2)] {
        for i in 0..16 {
            let mut px = rng.pixels(w * h + PAD);
            // Known, deliberately non-idempotent guard pattern.
            for (k, p) in px[w * h..].iter_mut().enumerate() {
                *p = CpPixel {
                    r: 0xDE,
                    g: 0xAD,
                    b: 0xBE,
                    a: (0x40 + k as u8),
                };
            }
            let out = diff(&px, w as i32, h as i32, &format!("row15 {w}x{h} iter {i}"));
            assert_eq!(
                bytes(&out[w * h..]),
                bytes(&px[w * h..]),
                "row15: guard pixels past the logical image must be untouched"
            );
        }
    }
}

// --- row 16 ----------------------------------------------------------------
#[test]
fn row16_unaligned_pix_pointer() {
    // cp_pixel_t has alignment 1, so a byte-offset pointer is a legal input.
    let l = libs();
    let mut rng = Rng::new(0x0000_0010);
    let (w, h) = (11usize, 5usize);
    for i in 0..32 {
        let src = rng.pixels(w * h);
        let src_bytes = bytes(&src).to_vec();

        let mut c_raw = vec![0u8; src_bytes.len() + 1];
        let mut r_raw = vec![0u8; src_bytes.len() + 1];
        c_raw[1..].copy_from_slice(&src_bytes);
        r_raw[1..].copy_from_slice(&src_bytes);

        let mut c_img = CpImage {
            w: w as i32,
            h: h as i32,
            pix: unsafe { c_raw.as_mut_ptr().add(1) } as *mut CpPixel,
        };
        let mut r_img = CpImage {
            w: w as i32,
            h: h as i32,
            pix: unsafe { r_raw.as_mut_ptr().add(1) } as *mut CpPixel,
        };
        unsafe {
            (l.c_premultiply)(&mut c_img);
            (l.rust_premultiply)(&mut r_img);
        }
        assert_eq!(c_raw, r_raw, "row16: unaligned pix diverged (iter {i})");
        assert_eq!(c_raw[0], 0, "row16: byte before pix must be untouched");
    }
}

// --- row 17 ----------------------------------------------------------------
#[test]
fn row17_repeated_application() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0011);
    let (w, h) = (13usize, 11usize);
    for i in 0..32 {
        let src = rng.pixels(w * h);
        let mut c_buf = src.clone();
        let mut r_buf = src.clone();
        let mut c_img = CpImage {
            w: w as i32,
            h: h as i32,
            pix: c_buf.as_mut_ptr(),
        };
        let mut r_img = CpImage {
            w: w as i32,
            h: h as i32,
            pix: r_buf.as_mut_ptr(),
        };
        for pass in 0..3 {
            unsafe {
                (l.c_premultiply)(&mut c_img);
                (l.rust_premultiply)(&mut r_img);
            }
            assert_eq!(
                bytes(&c_buf),
                bytes(&r_buf),
                "row17: diverged on pass {pass} (iter {i})"
            );
        }
    }
}

// --- row 18 ----------------------------------------------------------------
#[test]
fn row18_interleaved_independent_images() {
    let l = libs();
    let mut rng = Rng::new(0x0000_0012);
    for i in 0..32 {
        let a_src = rng.pixels(5 * 3);
        let b_src = rng.pixels(9 * 2);

        let (mut ca, mut ra) = (a_src.clone(), a_src.clone());
        let (mut cb, mut rb) = (b_src.clone(), b_src.clone());

        let mut cai = CpImage { w: 5, h: 3, pix: ca.as_mut_ptr() };
        let mut rai = CpImage { w: 5, h: 3, pix: ra.as_mut_ptr() };
        let mut cbi = CpImage { w: 9, h: 2, pix: cb.as_mut_ptr() };
        let mut rbi = CpImage { w: 9, h: 2, pix: rb.as_mut_ptr() };

        unsafe {
            (l.c_premultiply)(&mut cai);
            (l.c_premultiply)(&mut cbi);
            (l.c_premultiply)(&mut cai);
            (l.rust_premultiply)(&mut rai);
            (l.rust_premultiply)(&mut rbi);
            (l.rust_premultiply)(&mut rai);
        }
        assert_eq!(bytes(&ca), bytes(&ra), "row18: image A diverged (iter {i})");
        assert_eq!(bytes(&cb), bytes(&rb), "row18: image B diverged (iter {i})");
    }
}

// --- row 19 ----------------------------------------------------------------
#[test]
fn row19_zero_by_zero_leaves_buffer_untouched() {
    let mut rng = Rng::new(0x0000_0013);
    for i in 0..32 {
        let px = rng.pixels(16);
        let out = diff(&px, 0, 0, &format!("row19 iter {i}"));
        assert_eq!(
            bytes(&out),
            bytes(&px),
            "row19: w=h=0 must not touch the buffer"
        );
    }
}

// --- row 20 ----------------------------------------------------------------
#[test]
fn row20_one_dimension_empty() {
    let mut rng = Rng::new(0x0000_0014);
    for i in 0..32 {
        let px = rng.pixels(32);
        for (w, h) in [(0i32, 7i32), (0, 1), (0, 12345), (7, 0), (1, 0), (12345, 0)] {
            let out = diff(&px, w, h, &format!("row20 {w}x{h} iter {i}"));
            assert_eq!(
                bytes(&out),
                bytes(&px),
                "row20: empty shape {w}x{h} must not touch the buffer"
            );
        }
    }
}

// --- row 21 ----------------------------------------------------------------
#[test]
fn row21_negative_times_negative_does_work() {
    // stride = 4*w (negative), bound = stride*h (positive again) -> the C loop
    // really runs for `w*h` pixels. Buffer sized generously so both libs stay
    // inside the allocation while doing identical work.
    let mut rng = Rng::new(0x0000_0015);
    for (w, h) in [(-1i32, -1i32), (-2, -3), (-4, -5), (-1, -64), (-16, -2)] {
        let pixels = (w as i64 * h as i64) as usize;
        for i in 0..16 {
            let px = rng.pixels(pixels + 8);
            let out = diff(&px, w, h, &format!("row21 {w}x{h} iter {i}"));
            // Sanity: the first `pixels` entries were actually processed.
            assert_eq!(
                bytes(&out[pixels..]),
                bytes(&px[pixels..]),
                "row21: work must stop after {pixels} pixels"
            );
        }
    }
}

// --- row 22 ----------------------------------------------------------------
#[test]
fn row22_struct_fields_untouched() {
    // `diff` already asserts this on every call; assert it once explicitly for a
    // representative shape as well, including a shape that does work.
    let mut rng = Rng::new(0x0000_0016);
    for (w, h) in [(4i32, 4i32), (0, 0), (-1, -1), (7, 0)] {
        let n = (w as i64 * h as i64).unsigned_abs() as usize + 8;
        let px = rng.pixels(n);
        diff(&px, w, h, &format!("row22 {w}x{h}"));
    }
}

// --- row 23 ----------------------------------------------------------------
#[test]
fn row23_abi_layout() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<CpPixel>(), 4, "sizeof(cp_pixel_t)");
    assert_eq!(align_of::<CpPixel>(), 1, "alignof(cp_pixel_t)");
    assert_eq!(size_of::<CpImage>(), 16, "sizeof(cp_image_t) on LP64");
    assert_eq!(align_of::<CpImage>(), 8, "alignof(cp_image_t) on LP64");

    let probe = CpImage {
        w: 0,
        h: 0,
        pix: std::ptr::null_mut(),
    };
    let base = &probe as *const CpImage as usize;
    assert_eq!(&probe.w as *const _ as usize - base, 0, "offsetof(w)");
    assert_eq!(&probe.h as *const _ as usize - base, 4, "offsetof(h)");
    assert_eq!(&probe.pix as *const _ as usize - base, 8, "offsetof(pix)");

    // Round-trip a struct through both .so's: if either disagreed about the
    // field offsets, the dimensions would be misread and the byte compare in
    // `diff` would fail.
    let mut rng = Rng::new(0x0000_0017);
    let px = rng.pixels(6 * 6);
    diff_wh(&px, 6, 6, "row23 roundtrip");
}

// --- row 24 ----------------------------------------------------------------
#[test]
fn row24_full_i32_range_wrapping_with_null_pix() {
    // Sweep the wrapping arithmetic of `stride = w*4` and `bound = stride*h`
    // over the whole i32 range. `pix` is NULL, so only combinations whose bound
    // is <= 0 are safe to execute -- and those are precisely the ones that must
    // perform no dereference at all. Both libs must survive identically.
    let l = libs();
    let mut rng = Rng::new(0x0000_0018);
    let mut executed = 0usize;
    let mut skipped = 0usize;
    for _ in 0..2000 {
        let w = rng.next_i32();
        let h = rng.next_i32();
        let bound = w.wrapping_mul(4).wrapping_mul(h);
        if bound > 0 {
            skipped += 1;
            continue;
        }
        let mut c_img = CpImage { w, h, pix: std::ptr::null_mut() };
        let mut r_img = CpImage { w, h, pix: std::ptr::null_mut() };
        unsafe {
            (l.c_premultiply)(&mut c_img);
            (l.rust_premultiply)(&mut r_img);
        }
        assert_eq!((c_img.w, c_img.h), (w, h));
        assert_eq!((r_img.w, r_img.h), (w, h));
        assert!(c_img.pix.is_null() && r_img.pix.is_null());
        executed += 1;
    }
    assert!(executed > 100, "row24: too few executed cases ({executed})");
    // Also hammer the documented boundary values explicitly.
    for w in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for h in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            if w.wrapping_mul(4).wrapping_mul(h) > 0 {
                continue;
            }
            let mut c_img = CpImage { w, h, pix: std::ptr::null_mut() };
            let mut r_img = CpImage { w, h, pix: std::ptr::null_mut() };
            unsafe {
                (l.c_premultiply)(&mut c_img);
                (l.rust_premultiply)(&mut r_img);
            }
        }
    }
    eprintln!("row24: executed={executed} skipped(positive bound)={skipped}");
}
