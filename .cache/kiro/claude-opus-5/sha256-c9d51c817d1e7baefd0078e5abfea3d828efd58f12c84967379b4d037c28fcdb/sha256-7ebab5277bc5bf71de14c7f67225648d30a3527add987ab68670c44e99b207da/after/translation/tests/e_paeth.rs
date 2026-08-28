//! Exhaustive `cp_paeth` coverage, driven through `unfilter`'s filter type 4.
//!
//! With `bpp == 1` and two rows, row 1 computes
//! `out[x] = raw[x] + cp_paeth(out[x-1], prev[x], prev[x-1])`.
//! Laying `prev` out as `c, b, c, b, ...` makes the `(b, c)` argument pair
//! alternate between both orderings, and choosing `raw[x]` so that `out[x]`
//! walks 0..=255 sweeps the first argument. Iterating `(b, c)` over the full
//! 256x256 grid therefore visits every `(a, b, c)` triple.
//!
//! The local `paeth_model` is only used to *construct* inputs -- both libraries
//! receive byte-identical buffers, so a wrong model cannot mask a mismatch.

mod common;

use common::{libs, Aligned};
use std::os::raw::c_int;

fn paeth_model(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[test]
fn paeth_exhaustive_all_triples() {
    let l = libs();
    let w: c_int = 512;
    let h: c_int = 2;
    let bpp: c_int = 1;
    let len = (w * bpp) as usize;
    let stride = len + 1;
    let total = h as usize * stride;
    let guard = 16;

    let mut cbuf = Aligned::new(total + guard);
    let mut rbuf = Aligned::new(total + guard);
    let mut data = vec![0u8; total];

    // Track which (a, b, c) triples were actually driven, as a self-check that
    // the sweep really is exhaustive.
    let mut seen = vec![0u64; (1 << 24) / 64];
    let mut mark = |a: u8, b: u8, c: u8| {
        let i = ((a as usize) << 16) | ((b as usize) << 8) | c as usize;
        seen[i / 64] |= 1u64 << (i % 64);
    };

    for b in 0..256u32 {
        for c in 0..256u32 {
            // row 0: filter 0 (identity) -> becomes `prev`
            data[0] = 0;
            for x in 0..len {
                data[1 + x] = if x % 2 == 0 { c as u8 } else { b as u8 };
            }
            // row 1: filter 4
            data[stride] = 4;
            // x == 0 is the `x < bpp` loop: out[0] = raw[0] + prev[0]
            let want0 = 0u8;
            data[stride + 1] = want0.wrapping_sub(data[1]);
            let mut prev_out = want0;
            for x in 1..len {
                let a = prev_out;
                let bb = data[1 + x];
                let cc = data[1 + x - 1];
                mark(a, bb, cc);
                let want = (x / 2) as u8;
                data[stride + 1 + x] = want.wrapping_sub(paeth_model(a, bb, cc));
                prev_out = want;
            }

            cbuf.fill(&data);
            rbuf.fill(&data);
            let cret = unsafe { (l.c.unfilter)(w, h, bpp, cbuf.ptr()) };
            let rret = unsafe { (l.rust.unfilter)(w, h, bpp, rbuf.ptr()) };
            assert_eq!(cret, rret, "paeth sweep b={b} c={c}: return differs");
            if cbuf.as_slice() != rbuf.as_slice() {
                panic!(
                    "paeth sweep b={b} c={c}: buffer differs\n{}",
                    common::hexdiff(cbuf.as_slice(), rbuf.as_slice())
                );
            }
        }
    }

    let covered: u64 = seen.iter().map(|w| w.count_ones() as u64).sum();
    assert_eq!(
        covered,
        1u64 << 24,
        "sweep did not reach every (a, b, c) triple ({covered} of {})",
        1u64 << 24
    );
}
