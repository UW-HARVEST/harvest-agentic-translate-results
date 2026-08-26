//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` on the C shared library exactly):
//!   * `encode_quant`
//!
//! The translation is behaviour-preserving down to C's wrapping two's
//! complement integer arithmetic, truncating-toward-zero division and
//! arithmetic right shifts. Original quirks (e.g. the unused `p3`
//! variable and the fact that `d2` is compared against `d0` rather than
//! against the current best distortion) are reproduced verbatim -- no
//! bugs are "fixed".

#![allow(non_snake_case)]

use std::ffi::c_int;

/// Faithful translation of `encode_quant()` from `c_src/src/lib.c`.
///
/// ```c
/// int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn encode_quant(
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> c_int {
    // `int uni1, uni2;`
    // `int diff, p0, p1, p2, p3, d0, d1, d2, d3;`  (`p3` is never used in the C)
    let mut uni: i32 = uni;
    let mut uni1: i32;
    let mut uni2: i32;

    // uni1 = uni + 1;
    uni1 = uni.wrapping_add(1);
    // uni2 = uni - 1;
    uni2 = uni.wrapping_sub(1);

    // if ((uni ^ uni1) & (~7)) uni1 = uni;
    if (uni ^ uni1) & !7i32 != 0 {
        uni1 = uni;
    }
    // if ((uni ^ uni2) & (~7)) uni2 = uni;
    if (uni ^ uni2) & !7i32 != 0 {
        uni2 = uni;
    }

    if lsbit != 0 {
        if lsbit == 4 {
            // uni &= ~1; uni1 &= ~1; uni2 &= ~1;
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
            // uni |= (uni >> 1) & (uni >> 2) & 1;   (arithmetic shifts)
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if lsbit & 1 != 0 {
            // uni |= 1; uni1 |= 1; uni2 |= 1;
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            // uni &= ~1; uni1 &= ~1; uni2 &= ~1;
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
        }
    }

    // diff = ((2 * (uni & 7) + 1) * step) / 8;
    let mut diff: i32 = quant_diff(uni, step);
    // if (uni & 8) diff = -diff;
    if uni & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    // p0 = pred + diff;
    let p0: i32 = pred.wrapping_add(diff);
    // d0 = tgt - p0; d0 = d0 ^ (d0 >> 31);
    let mut d0: i32 = tgt.wrapping_sub(p0);
    d0 ^= d0 >> 31;

    // diff = ((2 * (uni1 & 7) + 1) * step) / 8;
    diff = quant_diff(uni1, step);
    // if (uni1 & 8) diff = -diff;
    if uni1 & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    // p1 = pred + diff;
    let p1: i32 = pred.wrapping_add(diff);
    // d1 = tgt - p1; d1 = d1 ^ (d1 >> 31);
    let mut d1: i32 = tgt.wrapping_sub(p1);
    d1 ^= d1 >> 31;

    // diff = ((2 * (uni2 & 7) + 1) * step) / 8;
    diff = quant_diff(uni2, step);
    // if (uni2 & 8) diff = -diff;
    if uni2 & 8 != 0 {
        diff = diff.wrapping_neg();
    }
    // p2 = pred + diff;
    let p2: i32 = pred.wrapping_add(diff);
    // d2 = tgt - p2; d2 = d2 ^ (d2 >> 31);
    let mut d2: i32 = tgt.wrapping_sub(p2);
    d2 ^= d2 >> 31;

    // d3 = tgt2 - p0; d3 = d3 ^ (d3 >> 31); d0 += d3 >> 5;
    let mut d3: i32 = tgt2.wrapping_sub(p0);
    d3 ^= d3 >> 31;
    d0 = d0.wrapping_add(d3 >> 5);

    // d3 = tgt2 - p1; d3 = d3 ^ (d3 >> 31); d1 += d3 >> 5;
    d3 = tgt2.wrapping_sub(p1);
    d3 ^= d3 >> 31;
    d1 = d1.wrapping_add(d3 >> 5);

    // d3 = tgt2 - p2; d3 = d3 ^ (d3 >> 31); d2 += d3 >> 5;
    d3 = tgt2.wrapping_sub(p2);
    d3 ^= d3 >> 31;
    d2 = d2.wrapping_add(d3 >> 5);

    // if (d1 < d0) uni = uni1;
    if d1 < d0 {
        uni = uni1;
    }
    // if (d2 < d0) uni = uni2;   (compared against d0, *not* the running best)
    if d2 < d0 {
        uni = uni2;
    }

    // return (uni);
    uni
}

/// `((2 * (u & 7) + 1) * step) / 8` with C semantics.
///
/// `u & 7` is always in `0..=7`, so `2 * (u & 7) + 1` (in `1..=15`) cannot
/// overflow; the multiplication by `step` can, and wraps like C on all
/// mainstream targets. Division by 8 truncates toward zero in both C and
/// Rust.
#[inline]
fn quant_diff(u: i32, step: i32) -> i32 {
    (2i32.wrapping_mul(u & 7).wrapping_add(1))
        .wrapping_mul(step)
        .wrapping_div(8)
}

#[cfg(test)]
mod tests {
    use super::encode_quant;

    #[test]
    fn matches_reference_on_small_grid() {
        // Reference values are produced by the straightforward reading of
        // the C source; a full differential test against the compiled C
        // library lives outside the crate.
        assert_eq!(encode_quant(0, 0, 0, 0, 0, 0), 0);
        assert_eq!(encode_quant(3, 16, 0, 100, 100, 0), 4);
        assert_eq!(encode_quant(3, 16, 0, 100, 100, 4), 4);
    }
}
