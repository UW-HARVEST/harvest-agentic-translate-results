//! Phase B — valid-path differential tests for `unfilter`.
//!
//! One test per row of `CONFIGS.md` §`unfilter` (rows 1..15).  Every row runs
//! many randomised inputs from a fixed seed, and every call goes through the
//! `.so`'s exported `unfilter` symbol on both sides.

mod common;

use common::{diff_unfilter, diff_unfilter_at, pair_asserts, pair_release, Pair};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

const SEED: u64 = 0x5EED_1234;

fn rng(row: u64) -> StdRng {
    StdRng::seed_from_u64(SEED ^ (row << 32))
}

/// `unfilter` contains no `assert()` and no allocation, so both C builds must
/// agree with Rust; we exercise both to be sure.
fn pairs() -> Vec<&'static Pair> {
    vec![pair_release(), pair_asserts()]
}

/// Buffer big enough for `h` rows of `1 + w*bpp` bytes plus generous slack,
/// because several rows deliberately make the C code index past `len`.
fn buf(r: &mut StdRng, w: i32, h: i32, bpp: i32) -> Vec<u8> {
    let rows = h.max(1) as usize + 2;
    let stride = 1 + (w.max(0) as usize) * (bpp.max(0) as usize);
    let n = rows * (stride + 16) + 64;
    (0..n).map(|_| r.gen::<u8>()).collect()
}

fn set_filters(b: &mut [u8], w: i32, h: i32, bpp: i32, f: impl Fn(usize) -> u8) {
    let stride = 1 + (w.max(0) as usize) * (bpp.max(0) as usize);
    for y in 0..h.max(0) as usize {
        let off = y * stride;
        if off < b.len() {
            b[off] = f(y);
        }
    }
}

// --------------------------------------------------------------------- row 1-5
// h == 1, every filter type, every bpp: the row-0 `switch` (lib.c:422).

fn single_row(row: u64, filter: u8) {
    let mut r = rng(row);
    for &bpp in &[1i32, 2, 3, 4] {
        for _ in 0..40 {
            let w = r.gen_range(1..=64);
            let mut b = buf(&mut r, w, 1, bpp);
            set_filters(&mut b, w, 1, bpp, |_| filter);
            for p in pairs() {
                diff_unfilter(p, &format!("row{row} f={filter} bpp={bpp} w={w}"), w, 1, bpp, &b);
            }
        }
    }
}

#[test]
fn row01_h1_filter_none() {
    single_row(1, 0);
}
#[test]
fn row02_h1_filter_sub() {
    single_row(2, 1);
}
#[test]
fn row03_h1_filter_up() {
    single_row(3, 2);
}
#[test]
fn row04_h1_filter_average() {
    single_row(4, 3);
}
#[test]
fn row05_h1_filter_paeth() {
    single_row(5, 4);
}

// ----------------------------------------------------------------------- row 6
// h == 2: first use of the `prev[]` path, exhaustive over (row-1 filter, bpp).

#[test]
fn row06_h2_first_prev_row() {
    let mut r = rng(6);
    for f1 in 0u8..=4 {
        for &bpp in &[1i32, 2, 3, 4] {
            for _ in 0..30 {
                let w = r.gen_range(1..=40);
                let mut b = buf(&mut r, w, 2, bpp);
                set_filters(&mut b, w, 2, bpp, |y| if y == 0 { 0 } else { f1 });
                for p in pairs() {
                    diff_unfilter(p, &format!("row6 f1={f1} bpp={bpp} w={w}"), w, 2, bpp, &b);
                }
            }
        }
    }
}

// ----------------------------------------------------------------------- row 7
// h in 3..8 with a random filter byte per row (A7 mixing) and prev chaining.

#[test]
fn row07_multirow_mixed_filters() {
    let mut r = rng(7);
    for _ in 0..400 {
        let h = r.gen_range(3..=8);
        let bpp = *[1i32, 2, 3, 4].iter().nth(r.gen_range(0..4)).unwrap();
        let w = r.gen_range(1..=48);
        let mut b = buf(&mut r, w, h, bpp);
        let filters: Vec<u8> = (0..h).map(|_| r.gen_range(0..=4u8)).collect();
        set_filters(&mut b, w, h, bpp, |y| filters[y]);
        for p in pairs() {
            diff_unfilter(
                p,
                &format!("row7 h={h} bpp={bpp} w={w} f={filters:?}"),
                w,
                h,
                bpp,
                &b,
            );
        }
    }
}

// ----------------------------------------------------------------------- row 8
// Byte values chosen to hit uint8_t wrap-around and all three paeth outcomes.

#[test]
fn row08_extreme_byte_values_paeth_branches() {
    let mut r = rng(8);
    const VALS: [u8; 6] = [0, 1, 127, 128, 254, 255];
    for _ in 0..300 {
        let h = r.gen_range(2..=6);
        let w = r.gen_range(1..=24);
        let bpp = 3;
        let mut b = buf(&mut r, w, h, bpp);
        for x in b.iter_mut() {
            *x = VALS[r.gen_range(0..VALS.len())];
        }
        let filters: Vec<u8> = (0..h).map(|_| r.gen_range(3..=4u8)).collect();
        set_filters(&mut b, w, h, bpp, |y| filters[y]);
        for p in pairs() {
            diff_unfilter(p, &format!("row8 h={h} w={w} f={filters:?}"), w, h, bpp, &b);
        }
    }
}

// ----------------------------------------------------------------------- row 9
// w == 1  =>  len == bpp: the `x < len` loops are empty, the row-y prologue
// exactly covers the row.

#[test]
fn row09_w1_len_equals_bpp() {
    let mut r = rng(9);
    for &bpp in &[1i32, 2, 3, 4] {
        for f in 0u8..=4 {
            for h in 1..=4 {
                let mut b = buf(&mut r, 1, h, bpp);
                set_filters(&mut b, 1, h, bpp, |_| f);
                for p in pairs() {
                    diff_unfilter(p, &format!("row9 bpp={bpp} f={f} h={h}"), 1, h, bpp, &b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- row 10
// w == 0  =>  len == 0.

#[test]
fn row10_w0_len_zero() {
    let mut r = rng(10);
    for f in 0u8..=4 {
        for h in 1..=5 {
            for &bpp in &[1i32, 3, 4] {
                let mut b = buf(&mut r, 0, h, bpp);
                set_filters(&mut b, 0, h, bpp, |_| f);
                for p in pairs() {
                    diff_unfilter(p, &format!("row10 f={f} h={h} bpp={bpp}"), 0, h, bpp, &b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- row 11
// bpp == 0  =>  len == 0.

#[test]
fn row11_bpp0_len_zero() {
    let mut r = rng(11);
    for f in 0u8..=4 {
        for w in 1..=8 {
            for h in 1..=4 {
                let mut b = buf(&mut r, w, h, 0);
                set_filters(&mut b, w, h, 0, |_| f);
                for p in pairs() {
                    diff_unfilter(p, &format!("row11 f={f} w={w} h={h}"), w, h, 0, &b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- row 12
// bpp > len: the row-y `x < bpp` prologue runs past `len` into the next row.

#[test]
fn row12_bpp_greater_than_len() {
    let mut r = rng(12);
    for bpp in 5i32..=9 {
        for f in 0u8..=4 {
            for h in 2..=4 {
                // w == 1 => len == bpp, so also try w == 0 => len == 0 < bpp
                for &w in &[0i32, 1] {
                    let mut b = buf(&mut r, w.max(1), h + 2, bpp);
                    set_filters(&mut b, w, h, bpp, |_| f);
                    for p in pairs() {
                        diff_unfilter(
                            p,
                            &format!("row12 bpp={bpp} f={f} h={h} w={w}"),
                            w,
                            h,
                            bpp,
                            &b,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- row 13
// h == 0 and h < 0 must ignore the buffer entirely (even a bogus filter byte).

#[test]
fn row13_h_zero_and_negative() {
    let mut r = rng(13);
    for &h in &[0i32, -1, -7, i32::MIN / 2] {
        for &bpp in &[1i32, 3, 4] {
            for &w in &[0i32, 1, 17] {
                let mut b = buf(&mut r, w, 1, bpp);
                for fb in [0u8, 3, 5, 200, 255] {
                    b[0] = fb;
                    for p in pairs() {
                        diff_unfilter(
                            p,
                            &format!("row13 h={h} w={w} bpp={bpp} fb={fb}"),
                            w,
                            h,
                            bpp,
                            &b,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- row 14
// negative w and/or bpp.

// `len = w*bpp` can come out negative here, which makes the C code's
// `raw += len` walk *backwards*; the buffer therefore has to be big and the
// pointer handed over has to sit in the middle of it.
#[test]
fn row14_negative_w_bpp() {
    let mut r = rng(14);
    const MID: usize = 4096;
    let mut b: Vec<u8> = (0..8192).map(|_| r.gen::<u8>()).collect();
    for &w in &[-1i32, -4, -33, 1, 32] {
        for &bpp in &[1i32, 3, 4, -1, -3] {
            if w > 0 && bpp > 0 {
                continue; // covered by rows 1..15
            }
            for h in 1..=3 {
                for fb in 0u8..=5 {
                    // set the filter byte of every row along the (possibly
                    // negative) stride
                    let stride = w.wrapping_mul(bpp);
                    for y in 0..h {
                        let off = MID as i64 + (y as i64) * (stride as i64 + 1);
                        if off >= 0 && (off as usize) < b.len() {
                            b[off as usize] = fb;
                        }
                    }
                    for p in pairs() {
                        diff_unfilter_at(
                            p,
                            &format!("row14 w={w} bpp={bpp} h={h} fb={fb}"),
                            w,
                            h,
                            bpp,
                            &b,
                            MID,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------- row 15
// Large images, deep Paeth runs.

#[test]
fn row15_large_paeth() {
    let mut r = rng(15);
    for _ in 0..12 {
        let w = r.gen_range(256..=1024);
        let h = r.gen_range(4..=16);
        let bpp = 4;
        let mut b = buf(&mut r, w, h, bpp);
        set_filters(&mut b, w, h, bpp, |_| 4);
        for p in pairs() {
            diff_unfilter(p, &format!("row15 w={w} h={h}"), w, h, bpp, &b);
        }
    }
    // and every filter on a wide image
    for f in 0u8..=4 {
        let (w, h, bpp) = (512i32, 8i32, 4i32);
        let mut b = buf(&mut r, w, h, bpp);
        set_filters(&mut b, w, h, bpp, |_| f);
        for p in pairs() {
            diff_unfilter(p, &format!("row15b f={f}"), w, h, bpp, &b);
        }
    }
}
