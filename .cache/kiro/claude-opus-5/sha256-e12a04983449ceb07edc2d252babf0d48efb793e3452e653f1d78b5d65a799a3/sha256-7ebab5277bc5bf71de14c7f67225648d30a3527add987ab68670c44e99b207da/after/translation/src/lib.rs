//! Rust translation of `c_src/src/lib.c` — colour-blindness (dichromacy)
//! simulation matrices applied in-place to a linear RGB triple.
//!
//! Behaviour is a faithful 1:1 port: the same `f32` arithmetic, in the same
//! order, with the same constants, and the same "unknown impairment values are
//! silently ignored" fallthrough as the original C `switch` (which has no
//! `default` label).
//!
//! # Why the sums are written with explicit `addss`/`subss` helpers
//!
//! For finite inputs the IEEE-754 `f32` result does not depend on the order in
//! which the three product terms are combined, so plain `a + b + c` would do.
//! NaN payloads are the exception: x86 `ADDSS`/`SUBSS` return the *destination*
//! (left) operand whenever it is NaN, and only fall back to the source operand
//! otherwise. Which term ends up on the left is a codegen choice, and it
//! differs between the C reference (GCC, `-O0`) and LLVM, which also rewrites
//! `sum - c*B` into `(-c)*B + sum` — flipping the winner.
//!
//! Encoding the operand order in the *data flow* rather than leaving it to the
//! optimiser makes the NaN result reproducible. Each expression below mirrors
//! the operand order of the compiled C, read off the disassembly.
//!
//! The helpers pass their result through [`core::hint::black_box`]. Without it
//! LLVM folds `isnan(a + b)` into `isnan(a) || isnan(b)` and collapses the
//! whole NaN path into a static "pick one of the three inputs" table, which
//! loses the default QNaN that `+inf + -inf` produces mid-chain. A couple of
//! register barriers in a nine-multiply function is a cheap price for matching
//! the reference bit-for-bit.

use std::ffi::c_int;
use std::hint::black_box;

/// Mirrors the C `enum cb_impairment` discriminants from `include/lib.h`.
pub const CB_PROTANOPIA: c_int = 0;
pub const CB_DEUTERANOPIA: c_int = 1;
pub const CB_TRITANOPIA: c_int = 2;

/// Quiet a NaN the way an x86 SSE arithmetic instruction does: set the
/// most-significant mantissa bit, leaving sign and payload untouched. An
/// already-quiet NaN comes back unchanged.
#[inline]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `ADDSS dst, src` — `dst + src`, with x86's NaN precedence made explicit:
/// a NaN destination wins over a NaN source.
///
/// When neither operand is NaN the plain addition is used; an invalid operation
/// such as `+inf + -inf` then yields the same default QNaN the hardware
/// produces, and being quiet it will win any later comparison here.
#[inline]
fn addss(dst: f32, src: f32) -> f32 {
    let result = if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst + src
    };
    // Opaque to the optimiser, so a caller's NaN test on this value is really
    // performed instead of being rewritten in terms of `dst`/`src`.
    black_box(result)
}

/// `SUBSS dst, src` — `dst - src`, with the same NaN precedence as [`addss`].
#[inline]
fn subss(dst: f32, src: f32) -> f32 {
    let result = if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else {
        dst - src
    };
    black_box(result)
}

/// `static void Protanopia(float *Red, float *Green, float *Blue)`
///
/// Operates on values, not pointers, so the load-all-then-store-all ordering of
/// the C original is preserved even when the caller passes aliasing pointers.
fn protanopia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        // *Red = 0.17055699213417f * R + 0.82944301379913f * G + 2.91188E-9f * B
        addss(
            2.91188E-9f32 * b,
            addss(r * 0.17055699213417f32, 0.82944301379913f32 * g),
        ),
        // *Green = 0.17055699092998f * R + 0.82944300785005f * G - 5.98679E-10f * B
        subss(
            addss(0.82944300785005f32 * g, r * 0.17055699092998f32),
            5.98679E-10f32 * b,
        ),
        // *Blue = -0.00451714424166f * R + 0.00451714427397f * G + B
        addss(
            addss(0.00451714427397f32 * g, r * -0.00451714424166f32),
            b,
        ),
    )
}

/// `static void Deuteranopia(float *Red, float *Green, float *Blue)`
fn deuteranopia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        // *Red = 0.33066007266046f * R + 0.66933992517563f * G + 3.559314E-9f * B
        addss(
            3.559314E-9f32 * b,
            addss(r * 0.33066007266046f32, 0.66933992517563f32 * g),
        ),
        // *Green = 0.33066007387760f * R + 0.66933992719147f * G - 1.758327E-9f * B
        subss(
            addss(0.66933992719147f32 * g, r * 0.33066007387760f32),
            1.758327E-9f32 * b,
        ),
        // *Blue = -0.02785538261323f * R + 0.02785538252318f * G + B
        addss(
            addss(0.02785538252318f32 * g, r * -0.02785538261323f32),
            b,
        ),
    )
}

/// `static void Tritanopia(float *Red, float *Green, float *Blue)`
fn tritanopia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        // *Red = R + 0.12739886310880f * G - 0.12739886341072f * B
        subss(
            addss(0.12739886310880f32 * g, r),
            0.12739886341072f32 * b,
        ),
        // *Green = -4.486E-11f * R + 0.87390929928361f * G + 0.12609070101523f * B
        addss(
            0.12609070101523f32 * b,
            addss(r * -4.486E-11f32, 0.87390929928361f32 * g),
        ),
        // *Blue = 3.1113E-10f * R + 0.87390929725848f * G + 0.12609070067115f * B
        addss(
            0.12609070067115f32 * b,
            addss(r * 3.1113E-10f32, 0.87390929725848f32 * g),
        ),
    )
}

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B)`
///
/// # Safety
///
/// `r`, `g` and `b` must each be valid, aligned, readable and writable pointers
/// to an `f32`, exactly as required by the C original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(impairment: c_int, r: *mut f32, g: *mut f32, b: *mut f32) {
    // The C code reads *Red, *Green, *Blue up front in every branch; for an
    // unrecognised impairment it dereferences nothing at all.
    let transform = match impairment {
        CB_PROTANOPIA => protanopia,
        CB_DEUTERANOPIA => deuteranopia,
        CB_TRITANOPIA => tritanopia,
        // No `default:` in the C switch: leave the values untouched.
        _ => return,
    };

    unsafe {
        let (nr, ng, nb) = transform(*r, *g, *b);
        *r = nr;
        *g = ng;
        *b = nb;
    }
}
