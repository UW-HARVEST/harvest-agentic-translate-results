//! Rust translation of `c_src/src/lib.c`.
//!
//! The C library exports exactly one public symbol: `encode_quant`
//! (verified with `nm -D` on the CMake-built shared object; the public header
//! `include/lib.h` declares only that function and contains no namespace
//! renaming macros).
//!
//! Semantics notes (kept faithful to the C, bugs included):
//! * All arithmetic is on C `int` == `i32`. Additions/subtractions/multiplications
//!   and negations use `wrapping_*` so that signed overflow wraps in two's
//!   complement exactly like the C compiler's generated code, instead of
//!   panicking in debug or being optimized away.
//! * `/` on `i32` in Rust truncates toward zero, matching C99 integer division.
//! * `>>` on `i32` in Rust is an arithmetic shift, matching the sign-propagating
//!   shift that `d0 >> 31` / `(uni >> 1)` rely on.
//! * The original declares `p3` but never uses it, and the final selection
//!   compares `d2 < d0` (not against the already-updated best). That behavior is
//!   reproduced verbatim rather than "fixed".

#![allow(non_snake_case)]

use std::ffi::c_int;

/// `int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit);`
#[unsafe(no_mangle)]
pub extern "C" fn encode_quant(
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> c_int {
    // C: int uni1, uni2;
    //    int diff, p0, p1, p2, p3, d0, d1, d2, d3;
    let mut uni: i32 = uni;
    let step: i32 = step;
    let pred: i32 = pred;
    let tgt: i32 = tgt;
    let tgt2: i32 = tgt2;
    let lsbit: i32 = lsbit;

    // uni1 = uni + 1;
    let mut uni1: i32 = uni.wrapping_add(1);
    // uni2 = uni - 1;
    let mut uni2: i32 = uni.wrapping_sub(1);

    // if ((uni ^ uni1) & (~7)) uni1 = uni;
    if ((uni ^ uni1) & !7i32) != 0 {
        uni1 = uni;
    }
    // if ((uni ^ uni2) & (~7)) uni2 = uni;
    if ((uni ^ uni2) & !7i32) != 0 {
        uni2 = uni;
    }

    if lsbit != 0 {
        if lsbit == 4 {
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if (lsbit & 1) != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1i32;
            uni1 &= !1i32;
            uni2 &= !1i32;
        }
    }

    // diff = ((2 * (uni & 7) + 1) * step) / 8;
    let mut diff: i32 = 2i32
        .wrapping_mul(uni & 7)
        .wrapping_add(1)
        .wrapping_mul(step)
        / 8;
    // if (uni & 8) diff = -diff;
    if (uni & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    let p0: i32 = pred.wrapping_add(diff);
    let mut d0: i32 = tgt.wrapping_sub(p0);
    d0 ^= d0 >> 31;

    diff = 2i32
        .wrapping_mul(uni1 & 7)
        .wrapping_add(1)
        .wrapping_mul(step)
        / 8;
    if (uni1 & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    let p1: i32 = pred.wrapping_add(diff);
    let mut d1: i32 = tgt.wrapping_sub(p1);
    d1 ^= d1 >> 31;

    diff = 2i32
        .wrapping_mul(uni2 & 7)
        .wrapping_add(1)
        .wrapping_mul(step)
        / 8;
    if (uni2 & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    let p2: i32 = pred.wrapping_add(diff);
    let mut d2: i32 = tgt.wrapping_sub(p2);
    d2 ^= d2 >> 31;

    let mut d3: i32 = tgt2.wrapping_sub(p0);
    d3 ^= d3 >> 31;
    d0 = d0.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p1);
    d3 ^= d3 >> 31;
    d1 = d1.wrapping_add(d3 >> 5);

    d3 = tgt2.wrapping_sub(p2);
    d3 ^= d3 >> 31;
    d2 = d2.wrapping_add(d3 >> 5);

    // NOTE: reproduces the original comparison chain verbatim (both candidates
    // are compared against the *original* d0, so uni2 wins ties with uni1).
    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }

    uni as c_int
}
