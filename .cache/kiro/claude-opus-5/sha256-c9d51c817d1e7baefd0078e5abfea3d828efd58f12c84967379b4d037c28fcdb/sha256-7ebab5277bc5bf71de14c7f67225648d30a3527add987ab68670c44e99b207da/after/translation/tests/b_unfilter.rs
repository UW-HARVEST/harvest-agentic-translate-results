//! `unfilter` -- the only function declared in `c_src/include/lib.h`.
//!
//! Every call goes through both `.so` files; the *whole* scratch buffer
//! (including a guard region past the rows) is compared byte-for-byte, so an
//! out-of-range write on either side shows up as a mismatch.

mod common;

use common::{libs, Aligned, Rng};
use std::os::raw::c_int;

/// Bytes the C code touches for `h` rows of `w * bpp` payload.
fn rows_len(w: c_int, h: c_int, bpp: c_int) -> usize {
    let len = w.wrapping_mul(bpp);
    if h <= 0 {
        0
    } else {
        (h as usize) * (len as usize + 1)
    }
}

/// Run `unfilter` on both libraries over identical input and compare the
/// return value plus the complete buffer.
fn check(w: c_int, h: c_int, bpp: c_int, seed_data: &[u8], label: &str) {
    let l = libs();
    let guard = 64;
    let total = seed_data.len() + guard;

    let mut cbuf = Aligned::new(total);
    let mut rbuf = Aligned::new(total);
    cbuf.fill(seed_data);
    rbuf.fill(seed_data);

    let cret = unsafe { (l.c.unfilter)(w, h, bpp, cbuf.ptr()) };
    let rret = unsafe { (l.rust.unfilter)(w, h, bpp, rbuf.ptr()) };

    assert_eq!(
        cret, rret,
        "{label}: return value differs (w={w} h={h} bpp={bpp})"
    );
    let a = cbuf.as_slice();
    let b = rbuf.as_slice();
    assert_eq!(
        a,
        b,
        "{label}: buffer differs (w={w} h={h} bpp={bpp})\n{}",
        common::hexdiff(a, b)
    );
}

/// Build a raw PNG-style scanline buffer: `h` rows of `1` filter byte followed
/// by `w * bpp` payload bytes.
fn make_rows(w: c_int, h: c_int, bpp: c_int, filters: &[u8], rng: &mut Rng) -> Vec<u8> {
    let len = (w * bpp) as usize;
    let mut v = Vec::with_capacity(rows_len(w, h, bpp));
    for y in 0..h.max(0) as usize {
        v.push(filters[y % filters.len()]);
        for _ in 0..len {
            v.push(rng.u8());
        }
    }
    v
}

#[test]
fn all_filter_types_single_and_multi_row() {
    let mut rng = Rng::new(0x1234_5678_9abc_def1);
    for bpp in 1..=4 {
        for w in [0, 1, 2, 3, 5, 8, 17] {
            for h in [0, 1, 2, 3, 7] {
                // uniform filter per whole image
                for f in 0u8..=4 {
                    let data = make_rows(w, h, bpp, &[f], &mut rng);
                    check(w, h, bpp, &data, &format!("uniform filter {f}"));
                }
                // mixed filters, every row a different one
                let data = make_rows(w, h, bpp, &[0, 1, 2, 3, 4, 3, 2, 1], &mut rng);
                check(w, h, bpp, &data, "mixed filters");
            }
        }
    }
}

#[test]
fn invalid_filter_byte_every_position() {
    let mut rng = Rng::new(0xdead_beef_cafe_0001);
    // An unknown filter byte makes the C code bail out with 0 -- possibly after
    // having already rewritten earlier rows.
    for bad in [5u8, 6, 7, 63, 127, 128, 200, 255] {
        for h in 1..=5 {
            for bad_row in 0..h {
                let w = 6;
                let bpp = 3;
                let mut data = make_rows(w, h, bpp, &[1, 2, 3, 4, 0], &mut rng);
                let stride = (w * bpp + 1) as usize;
                data[bad_row as usize * stride] = bad;
                check(
                    w,
                    h,
                    bpp,
                    &data,
                    &format!("bad filter {bad} at row {bad_row}/{h}"),
                );
            }
        }
    }
}

#[test]
fn saturating_and_extreme_payloads() {
    // All-0x00, all-0xFF, and 0x80 patterns stress the wrap-around in
    // `raw[x] += ...` and the sign handling inside `cp_paeth`.
    for fill in [0x00u8, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
        for f in 0u8..=4 {
            for bpp in 1..=4 {
                let w = 9;
                let h = 5;
                let stride = (w * bpp + 1) as usize;
                let mut data = vec![fill; h as usize * stride];
                for y in 0..h as usize {
                    data[y * stride] = f;
                }
                check(w, h, bpp, &data, &format!("fill {fill:#04x} filter {f}"));
            }
        }
    }
}

#[test]
fn paeth_predictor_exhaustive_via_filter4() {
    // Filter 4 on a 2-row, bpp=1 image lets us drive cp_paeth(a, b, c) over the
    // full 24-bit input space in strides: row0 supplies c/a, row1 supplies b.
    let l = libs();
    let w: c_int = 256;
    let h: c_int = 2;
    let bpp: c_int = 1;
    let stride = (w * bpp + 1) as usize;

    for c_val in 0u32..256 {
        for a_val in [0u32, 1, 63, 64, 127, 128, 129, 200, 254, 255] {
            let mut data = vec![0u8; h as usize * stride];
            // row 0: filter 0 (no-op) -> becomes `prev`
            data[0] = 0;
            for x in 0..w as usize {
                // prev[x] = c_val for x-1 lookups, and prev[x] itself
                data[1 + x] = if x == 0 { c_val as u8 } else { c_val as u8 };
            }
            // row 1: filter 4
            data[stride] = 4;
            for x in 0..w as usize {
                data[stride + 1 + x] = x as u8;
            }
            // Nudge the first payload byte so `a` (raw[x-bpp]) sweeps too.
            data[stride + 1] = a_val as u8;

            let guard = 32;
            let mut cbuf = Aligned::new(data.len() + guard);
            let mut rbuf = Aligned::new(data.len() + guard);
            cbuf.fill(&data);
            rbuf.fill(&data);
            let cret = unsafe { (l.c.unfilter)(w, h, bpp, cbuf.ptr()) };
            let rret = unsafe { (l.rust.unfilter)(w, h, bpp, rbuf.ptr()) };
            assert_eq!(cret, rret);
            assert_eq!(
                cbuf.as_slice(),
                rbuf.as_slice(),
                "paeth sweep c={c_val} a={a_val}\n{}",
                common::hexdiff(cbuf.as_slice(), rbuf.as_slice())
            );
        }
    }
}

#[test]
fn randomised_fuzz() {
    let mut rng = Rng::new(0x0bad_c0de_0000_0007);
    for iter in 0..4000 {
        let w = rng.below(20) as c_int;
        let h = rng.below(9) as c_int;
        let bpp = 1 + rng.below(4) as c_int;
        let len = (w * bpp) as usize;
        let mut data = Vec::with_capacity(rows_len(w, h, bpp));
        for _ in 0..h.max(0) {
            // mostly-valid filter bytes with occasional garbage
            let f = if rng.below(8) == 0 {
                rng.u8()
            } else {
                rng.below(5) as u8
            };
            data.push(f);
            for _ in 0..len {
                data.push(rng.u8());
            }
        }
        check(w, h, bpp, &data, &format!("fuzz iter {iter}"));
    }
}

#[test]
fn large_image() {
    let mut rng = Rng::new(0xfeed_face_1111_2222);
    for f in 0u8..=4 {
        let w = 257;
        let h = 61;
        let bpp = 4;
        let data = make_rows(w, h, bpp, &[f], &mut rng);
        check(w, h, bpp, &data, &format!("large filter {f}"));
    }
    let data = make_rows(200, 40, 3, &[0, 1, 2, 3, 4], &mut rng);
    check(200, 40, 3, &data, "large mixed");
}

#[test]
fn zero_and_degenerate_dimensions() {
    let mut rng = Rng::new(0xabcd_0000_ffff_0001);
    // w == 0 -> len == 0, each row is just its filter byte
    for h in 0..=6 {
        for f in 0u8..=5 {
            let data = make_rows(0, h, 3, &[f], &mut rng);
            check(0, h, 3, &data, &format!("w=0 h={h} f={f}"));
        }
    }
    // h == 0 must not touch the buffer at all
    for w in [0, 1, 9] {
        for bpp in 1..=4 {
            let data: Vec<u8> = (0..64).map(|_| rng.u8()).collect();
            check(w, 0, bpp, &data, "h=0");
        }
    }
    // h < 0 behaves like h == 0 (the `h > 0` guard and `y < h` both fail)
    for w in [0, 1, 9] {
        let data: Vec<u8> = (0..64).map(|_| rng.u8()).collect();
        check(w, -1, 2, &data, "h=-1");
        check(w, -7, 3, &data, "h=-7");
    }
    // bpp >= len: the inner `x < len` loops never run
    for bpp in [5, 8, 16, 32] {
        let data = make_rows(1, 4, bpp, &[0, 1, 2, 3, 4], &mut rng);
        check(1, 4, bpp, &data, &format!("bpp={bpp} >= w"));
    }
}
