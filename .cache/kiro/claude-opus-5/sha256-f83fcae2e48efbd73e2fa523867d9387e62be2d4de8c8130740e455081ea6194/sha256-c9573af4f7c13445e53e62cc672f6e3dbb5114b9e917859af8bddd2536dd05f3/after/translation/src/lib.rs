//! Rust translation of the C colour-blindness simulation library in `c_src/`.
//!
//! # Exported ABI
//!
//! `nm -D` on the C shared object shows exactly one exported function,
//! `colourblind`. The three per-impairment matrix transforms in `src/lib.c` are
//! declared `static`, so they are not part of the exported ABI; they are kept
//! private here as well. The public header declares no namespace/renaming
//! macros, so the linker symbol is the source-level name.
//!
//! # Bit-exactness
//!
//! Every arithmetic step is a single-precision multiply, add or subtract, which
//! on the C target (x86-64, SSE, no FMA contraction) rounds to `f32` after each
//! operation — the same semantics as Rust's `f32` operators. Rust also shares
//! C's operator precedence and left-to-right associativity, so the results agree
//! bit-for-bit for all finite and infinite inputs.
//!
//! The one place where plain Rust operators are *not* enough is the payload and
//! sign of a NaN *result*. Those bits are not fixed by IEEE 754; they fall out of
//! which operand a machine instruction happens to use as its destination
//! register, and LLVM considers the choice non-semantic and freely commutes
//! operands. To match the C library exactly on NaN inputs, the [`mulss`],
//! [`addss`] and [`subss`] helpers below implement x86 SSE's NaN selection rules
//! explicitly, and each expression is transcribed with the same destination /
//! source operand roles the reference C build emits. See the helper docs.

// The crate name mirrors the C shared object's name so the built artifact lines
// up with it; that spelling is not snake case.
#![allow(non_snake_case)]

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// x86 SSE scalar float semantics
// ---------------------------------------------------------------------------

/// The x86 "real indefinite" QNaN produced when an SSE operation on non-NaN
/// operands raises the invalid-operation exception (`inf - inf`, `0 * inf`).
const X86_DEFAULT_QNAN: u32 = 0xFFC0_0000;

/// Quiet a NaN the way SSE does when it forwards an operand: set the
/// most-significant mantissa bit, preserving sign and the rest of the payload.
#[inline]
fn quiet_nan(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// Apply SSE's result-selection rules for a scalar arithmetic instruction
/// `op dst, src`, given the IEEE result `computed`.
///
/// SSE forwards `dst` if it is NaN, otherwise `src` if it is NaN, otherwise it
/// returns the computed value — substituting the default QNaN if the operation
/// itself was invalid. The `dst`-before-`src` priority is why operand roles have
/// to be preserved when both operands can be NaN.
#[inline]
fn sse_scalar(dst: f32, src: f32, computed: f32) -> f32 {
    if dst.is_nan() {
        quiet_nan(dst)
    } else if src.is_nan() {
        quiet_nan(src)
    } else if computed.is_nan() {
        f32::from_bits(X86_DEFAULT_QNAN)
    } else {
        computed
    }
}

/// `mulss dst, src` — returns `dst * src`.
#[inline]
fn mulss(dst: f32, src: f32) -> f32 {
    sse_scalar(dst, src, dst * src)
}

/// `addss dst, src` — returns `dst + src`.
#[inline]
fn addss(dst: f32, src: f32) -> f32 {
    sse_scalar(dst, src, dst + src)
}

/// `subss dst, src` — returns `dst - src`.
#[inline]
fn subss(dst: f32, src: f32) -> f32 {
    sse_scalar(dst, src, dst - src)
}

// ---------------------------------------------------------------------------
// Impairment discriminants
// ---------------------------------------------------------------------------

// `enum cb_impairment` in `include/lib.h` gives no explicit values, so the
// enumerators are 0, 1 and 2 and the argument is passed as a plain `int`.
const CB_PROTANOPIA: c_int = 0;
const CB_DEUTERANOPIA: c_int = 1;
const CB_TRITANOPIA: c_int = 2;

// ---------------------------------------------------------------------------
// Matrix coefficients, transcribed verbatim from src/lib.c
// ---------------------------------------------------------------------------

// Note: within each transform several literals differ only past the 7th
// significant digit and therefore round to the same `f32` (the C compiler
// likewise emits a single pooled constant for them). They are still written out
// individually here so the source mirrors the C line for line.

// Protanopia
const P_RR: f32 = 0.17055699213417;
const P_RG: f32 = 0.82944301379913;
const P_RB: f32 = 2.91188E-9;
const P_GR: f32 = 0.17055699092998;
const P_GG: f32 = 0.82944300785005;
const P_GB: f32 = 5.98679E-10;
const P_BR: f32 = -0.00451714424166;
const P_BG: f32 = 0.00451714427397;

// Deuteranopia
const D_RR: f32 = 0.33066007266046;
const D_RG: f32 = 0.66933992517563;
const D_RB: f32 = 3.559314E-9;
const D_GR: f32 = 0.33066007387760;
const D_GG: f32 = 0.66933992719147;
const D_GB: f32 = 1.758327E-9;
const D_BR: f32 = -0.02785538261323;
const D_BG: f32 = 0.02785538252318;

// Tritanopia
const T_RG: f32 = 0.12739886310880;
const T_RB: f32 = 0.12739886341072;
const T_GR: f32 = -4.486E-11;
const T_GG: f32 = 0.87390929928361;
const T_GB: f32 = 0.12609070101523;
const T_BR: f32 = 3.1113E-10;
const T_BG: f32 = 0.87390929725848;
const T_BB: f32 = 0.12609070067115;

// ---------------------------------------------------------------------------
// The three transforms (`static` in C, private here)
// ---------------------------------------------------------------------------

/// `static void Protanopia(float *Red, float *Green, float *Blue)`
///
/// # Safety
/// `red`, `green` and `blue` must each be valid for reads and writes of an
/// `f32`, exactly as the C function requires of its arguments.
unsafe fn protanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    // float R = *Red, G = *Green, B = *Blue;
    let (r, g, b) = unsafe { (*red, *green, *blue) };

    // *Red = P_RR * R + P_RG * G + P_RB * B;
    let out_r = {
        let t1 = mulss(r, P_RR);
        let t2 = mulss(P_RG, g);
        let t3 = addss(t1, t2);
        let t4 = mulss(P_RB, b);
        addss(t4, t3)
    };

    // *Green = P_GR * R + P_GG * G - P_GB * B;
    let out_g = {
        let t1 = mulss(r, P_GR);
        let t2 = mulss(P_GG, g);
        let t3 = addss(t2, t1);
        let t4 = mulss(P_GB, b);
        subss(t3, t4)
    };

    // *Blue = P_BR * R + P_BG * G + B;
    let out_b = {
        let t1 = mulss(r, P_BR);
        let t2 = mulss(P_BG, g);
        let t3 = addss(t2, t1);
        addss(t3, b)
    };

    unsafe {
        *red = out_r;
        *green = out_g;
        *blue = out_b;
    }
}

/// `static void Deuteranopia(float *Red, float *Green, float *Blue)`
///
/// # Safety
/// `red`, `green` and `blue` must each be valid for reads and writes of an
/// `f32`, exactly as the C function requires of its arguments.
unsafe fn deuteranopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    // float R = *Red, G = *Green, B = *Blue;
    let (r, g, b) = unsafe { (*red, *green, *blue) };

    // *Red = D_RR * R + D_RG * G + D_RB * B;
    let out_r = {
        let t1 = mulss(r, D_RR);
        let t2 = mulss(D_RG, g);
        let t3 = addss(t1, t2);
        let t4 = mulss(D_RB, b);
        addss(t4, t3)
    };

    // *Green = D_GR * R + D_GG * G - D_GB * B;
    let out_g = {
        let t1 = mulss(r, D_GR);
        let t2 = mulss(D_GG, g);
        let t3 = addss(t2, t1);
        let t4 = mulss(D_GB, b);
        subss(t3, t4)
    };

    // *Blue = D_BR * R + D_BG * G + B;
    let out_b = {
        let t1 = mulss(r, D_BR);
        let t2 = mulss(D_BG, g);
        let t3 = addss(t2, t1);
        addss(t3, b)
    };

    unsafe {
        *red = out_r;
        *green = out_g;
        *blue = out_b;
    }
}

/// `static void Tritanopia(float *Red, float *Green, float *Blue)`
///
/// # Safety
/// `red`, `green` and `blue` must each be valid for reads and writes of an
/// `f32`, exactly as the C function requires of its arguments.
unsafe fn tritanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    // float R = *Red, G = *Green, B = *Blue;
    let (r, g, b) = unsafe { (*red, *green, *blue) };

    // *Red = R + T_RG * G - T_RB * B;
    let out_r = {
        let t1 = mulss(T_RG, g);
        let t2 = addss(t1, r);
        let t3 = mulss(T_RB, b);
        subss(t2, t3)
    };

    // *Green = T_GR * R + T_GG * G + T_GB * B;
    let out_g = {
        let t1 = mulss(r, T_GR);
        let t2 = mulss(T_GG, g);
        let t3 = addss(t1, t2);
        let t4 = mulss(T_GB, b);
        addss(t4, t3)
    };

    // *Blue = T_BR * R + T_BG * G + T_BB * B;
    let out_b = {
        let t1 = mulss(r, T_BR);
        let t2 = mulss(T_BG, g);
        let t3 = addss(t1, t2);
        let t4 = mulss(T_BB, b);
        addss(t4, t3)
    };

    unsafe {
        *red = out_r;
        *green = out_g;
        *blue = out_b;
    }
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B)`
///
/// Applies the requested colour-blindness simulation matrix in place. As in the
/// C original the `switch` carries no `default` label, so an `Impairment` value
/// outside the enumeration leaves `*R`, `*G` and `*B` untouched (and does not
/// dereference the pointers at all).
///
/// # Safety
/// When `impairment` names one of the three impairments, `r`, `g` and `b` must
/// each be valid for reads and writes of an `f32`; the C code dereferences them
/// unconditionally in that case.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(impairment: c_int, r: *mut f32, g: *mut f32, b: *mut f32) {
    match impairment {
        CB_PROTANOPIA => unsafe { protanopia(r, g, b) },
        CB_DEUTERANOPIA => unsafe { deuteranopia(r, g, b) },
        CB_TRITANOPIA => unsafe { tritanopia(r, g, b) },
        // The C switch has no `default:` case.
        _ => {}
    }
}
