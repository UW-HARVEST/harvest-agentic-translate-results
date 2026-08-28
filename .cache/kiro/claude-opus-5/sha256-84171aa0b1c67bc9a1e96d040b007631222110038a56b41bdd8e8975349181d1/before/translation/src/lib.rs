//! Rust translation of `c_src/src/lib.c`.
//!
//! The public entry point `encode_quant` mirrors the C implementation
//! bit-for-bit. The header (`c_src/include/lib.h`) declares no namespace
//! renaming macros, so the final linker symbol is plain `encode_quant`.
//!
//! All integer arithmetic uses explicit wrapping operations so that the
//! behaviour matches what the C compiler emits for `int` math on a two's
//! complement target (the C standard calls signed overflow UB; gcc/clang
//! wrap in practice). Right shifts on `i32` are arithmetic in Rust, which
//! matches the sign-propagating shifts the C code relies on for its
//! branchless `abs` idiom (`d ^ (d >> 31)`).

use std::ffi::c_int;

/// Branch-free "absolute value" exactly as written in the C source:
/// `d = d ^ (d >> 31)`. Note this yields `abs(d) - 1` for negative `d`,
/// which is the original behaviour and is deliberately preserved.
#[inline]
fn xor_abs(d: c_int) -> c_int {
    d ^ (d >> 31)
}

/// `diff = ((2 * (u & 7) + 1) * step) / 8`, negated when bit 3 of `u` is set.
#[inline]
fn quant_diff(u: c_int, step: c_int) -> c_int {
    let diff = ((2i32.wrapping_mul(u & 7)).wrapping_add(1))
        .wrapping_mul(step)
        .wrapping_div(8);
    if u & 8 != 0 { diff.wrapping_neg() } else { diff }
}

#[unsafe(no_mangle)]
pub extern "C" fn encode_quant(
    uni: c_int,
    step: c_int,
    pred: c_int,
    tgt: c_int,
    tgt2: c_int,
    lsbit: c_int,
) -> c_int {
    let mut uni = uni;

    let mut uni1 = uni.wrapping_add(1);
    let mut uni2 = uni.wrapping_sub(1);

    if (uni ^ uni1) & !7 != 0 {
        uni1 = uni;
    }
    if (uni ^ uni2) & !7 != 0 {
        uni2 = uni;
    }

    if lsbit != 0 {
        if lsbit == 4 {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
            uni |= (uni >> 1) & (uni >> 2) & 1;
            uni1 |= (uni1 >> 1) & (uni1 >> 2) & 1;
            uni2 |= (uni2 >> 1) & (uni2 >> 2) & 1;
        } else if lsbit & 1 != 0 {
            uni |= 1;
            uni1 |= 1;
            uni2 |= 1;
        } else {
            uni &= !1;
            uni1 &= !1;
            uni2 &= !1;
        }
    }

    let p0 = pred.wrapping_add(quant_diff(uni, step));
    let mut d0 = xor_abs(tgt.wrapping_sub(p0));

    let p1 = pred.wrapping_add(quant_diff(uni1, step));
    let mut d1 = xor_abs(tgt.wrapping_sub(p1));

    let p2 = pred.wrapping_add(quant_diff(uni2, step));
    let mut d2 = xor_abs(tgt.wrapping_sub(p2));

    let mut d3 = xor_abs(tgt2.wrapping_sub(p0));
    d0 = d0.wrapping_add(d3 >> 5);

    d3 = xor_abs(tgt2.wrapping_sub(p1));
    d1 = d1.wrapping_add(d3 >> 5);

    d3 = xor_abs(tgt2.wrapping_sub(p2));
    d2 = d2.wrapping_add(d3 >> 5);

    // Both comparisons are against the original d0, and uni2 wins ties with
    // uni1 when both are better -- preserved as in the C source.
    if d1 < d0 {
        uni = uni1;
    }
    if d2 < d0 {
        uni = uni2;
    }

    uni
}
