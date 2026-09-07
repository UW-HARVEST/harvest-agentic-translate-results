//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI surface (matches `nm -D` on the C shared object):
//!   * `encode_quant`
//!
//! Semantics are reproduced exactly, including the original code's quirks (for
//! example both candidate distortions `d1`/`d2` are compared against the
//! *original* `d0` rather than against a running best). All arithmetic uses
//! wrapping operations so that the two's-complement behaviour emitted by the C
//! compiler is preserved rather than panicking in debug builds.

use std::ffi::c_int;

/// Translation of `int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit)`
/// from `c_src/src/lib.c`.
#[unsafe(no_mangle)]
pub extern "C" fn encode_quant(
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> c_int {
    let mut uni: i32 = uni;
    let step: i32 = step;
    let pred: i32 = pred;
    let tgt: i32 = tgt;
    let tgt2: i32 = tgt2;
    let lsbit: i32 = lsbit;

    let mut uni1: i32;
    let mut uni2: i32;
    let mut diff: i32;
    let p0: i32;
    let p1: i32;
    let p2: i32;
    let mut d0: i32;
    let mut d1: i32;
    let mut d2: i32;
    let mut d3: i32;

    // uni1 = uni + 1; uni2 = uni - 1;
    uni1 = uni.wrapping_add(1);
    uni2 = uni.wrapping_sub(1);

    // Reject candidates that cross a 3-bit magnitude boundary.
    if ((uni ^ uni1) & !7) != 0 {
        uni1 = uni;
    }
    if ((uni ^ uni2) & !7) != 0 {
        uni2 = uni;
    }

    // Least-significant-bit conditioning.
    if lsbit != 0 {
        if lsbit == 4 {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if (lsbit & 1) != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
        }
    }

    // Candidate 0: uni
    diff = (2i32
        .wrapping_mul(uni & 7)
        .wrapping_add(1)
        .wrapping_mul(step))
        / 8;
    if (uni & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    p0 = pred.wrapping_add(diff);
    d0 = tgt.wrapping_sub(p0);
    d0 ^= d0 >> 31;

    // Candidate 1: uni1
    diff = (2i32
        .wrapping_mul(uni1 & 7)
        .wrapping_add(1)
        .wrapping_mul(step))
        / 8;
    if (uni1 & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    p1 = pred.wrapping_add(diff);
    d1 = tgt.wrapping_sub(p1);
    d1 ^= d1 >> 31;

    // Candidate 2: uni2
    diff = (2i32
        .wrapping_mul(uni2 & 7)
        .wrapping_add(1)
        .wrapping_mul(step))
        / 8;
    if (uni2 & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    p2 = pred.wrapping_add(diff);
    d2 = tgt.wrapping_sub(p2);
    d2 ^= d2 >> 31;

    // Secondary target contributes a 1/32-weighted penalty.
    d3 = tgt2.wrapping_sub(p0);
    d3 ^= d3 >> 31;
    d0 = d0.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p1);
    d3 ^= d3 >> 31;
    d1 = d1.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p2);
    d3 ^= d3 >> 31;
    d2 = d2.wrapping_add(d3 >> 5);

    // NOTE: both comparisons are against the original d0, exactly as in the C.
    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }

    uni
}
