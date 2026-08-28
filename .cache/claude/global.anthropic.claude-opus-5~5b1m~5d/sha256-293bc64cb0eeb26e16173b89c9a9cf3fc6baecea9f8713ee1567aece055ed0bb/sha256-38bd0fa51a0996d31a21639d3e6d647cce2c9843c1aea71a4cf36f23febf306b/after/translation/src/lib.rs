//! Rust translation of the `colourblind` C library (c_src/src/lib.c, c_src/include/lib.h).
//!
//! Public ABI, as reported by `nm -D` on the C shared object: a single exported
//! function, `colourblind`. The three per-impairment transform helpers are
//! `static` in the C source and therefore intentionally NOT exported here.
//!
//! # Behaviour preserved exactly from the C
//!
//! * Each helper reads all three components into locals *before* writing any of
//!   them back, then stores in the order Red, Green, Blue. This is observable
//!   when the caller passes aliasing pointers, so raw pointer reads/writes in
//!   that exact order are used rather than a safe by-value abstraction.
//! * The `switch` in `colourblind` has no `default` arm, so an impairment value
//!   outside {0, 1, 2} is a silent no-op. The C compiler compares the enum as an
//!   unsigned int, so out-of-range and "negative" values are no-ops too.
//! * No NULL checks: the C dereferences unconditionally, so this does too.
//! * All arithmetic is IEEE-754 single precision evaluated left-to-right, with
//!   no fused multiply-add contraction and no promotion to double.
//!
//! # Why the arithmetic goes through `sse`
//!
//! For finite inputs, plain `f32` operators already reproduce the C bit-for-bit.
//! They differ only in the *sign of a NaN result* when two NaN operands with
//! different signs meet in one expression: x86's `ADDSS`/`SUBSS`/`MULSS` return
//! the **destination** operand in that case, so which NaN survives depends on
//! the operand order the compiler happened to pick. LLVM canonicalises those
//! operands differently from GCC (and rewrites `a - c*b` into an add of a
//! negated constant), which flips some NaN sign bits.
//!
//! To stay byte-identical the operand order of every add/sub/mul below is
//! transcribed from the reference build's assembly (`gcc -S -O0`, which is what
//! the project's CMakeLists produces since it sets no `CMAKE_BUILD_TYPE`). The
//! `sse` helpers take their arguments in `(dst, src)` order, mirroring Intel
//! syntax, so each call site reads the same way as the instruction it stands for.

// C-style identifiers are kept verbatim so the Rust surface mirrors lib.h.
#![allow(non_camel_case_types, non_upper_case_globals)]

use std::ffi::c_uint;

/// Scalar single-precision ops with a pinned operand order.
///
/// `dst` is the operand that wins when both operands are NaN, matching
/// `MULSS`/`ADDSS`/`SUBSS`. On x86 these compile to exactly that instruction;
/// elsewhere they fall back to plain operators, which are identical for every
/// input except same-expression NaN-sign ties.
mod sse {
    /// Emits one scalar SSE instruction with `dst` and `src` in exactly the
    /// written order.
    ///
    /// The `_mm_*_ss` intrinsics are not usable here: LLVM folds them back into
    /// scalar `fadd`/`fmul`, then commutes and re-associates the operands (and
    /// rewrites `x - k*y` into an add of a negated constant), which is precisely
    /// the operand order this module exists to pin down. An `asm!` block is
    /// opaque to those rewrites, so the instruction survives verbatim.
    ///
    /// `pure` + `nomem` still let the compiler CSE and drop redundant copies,
    /// which is harmless: equal inputs always give equal results.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    macro_rules! scalar_op {
        ($name:ident, $mnemonic:literal) => {
            #[doc = concat!("`", $mnemonic, " dst, src`")]
            #[inline(always)]
            pub fn $name(dst: f32, src: f32) -> f32 {
                let mut d = dst;
                // SAFETY: SSE is part of the x86-64 baseline, so this
                // instruction is always available. It only reads/writes the two
                // named registers, touches no memory and clobbers no flags,
                // matching the declared options.
                unsafe {
                    core::arch::asm!(
                        concat!($mnemonic, " {d}, {s}"),
                        d = inout(xmm_reg) d,
                        s = in(xmm_reg) src,
                        options(pure, nomem, nostack, preserves_flags),
                    );
                }
                d
            }
        };
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    scalar_op!(mul, "mulss");
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    scalar_op!(add, "addss");
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    scalar_op!(sub, "subss");

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub fn mul(dst: f32, src: f32) -> f32 {
        dst * src
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub fn add(dst: f32, src: f32) -> f32 {
        dst + src
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    #[inline(always)]
    pub fn sub(dst: f32, src: f32) -> f32 {
        dst - src
    }
}

use sse::{add, mul, sub};

/// `typedef enum cb_impairment { cbProtanopia, cbDeuteranopia, cbTritanopia }`
///
/// The C enum's values are 0, 1 and 2; the compiler picks `unsigned int` as its
/// compatible type, which is what the ABI passes in the first integer register.
pub type cb_impairment = c_uint;

pub const cbProtanopia: cb_impairment = 0;
pub const cbDeuteranopia: cb_impairment = 1;
pub const cbTritanopia: cb_impairment = 2;

/// ```c
/// static void Protanopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = 0.17055699213417f * R + 0.82944301379913f * G + 2.91188E-9f * B;
///     *Green = 0.17055699092998f * R + 0.82944300785005f * G - 5.98679E-10f * B;
///     *Blue = -0.00451714424166f * R + 0.00451714427397f * G + B;
/// }
/// ```
unsafe fn protanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = (*red, *green, *blue);

    // t3 + (t1 + t2)
    let t1 = mul(r, 0.17055699213417f32);
    let t2 = mul(0.82944301379913f32, g);
    let t3 = mul(2.91188E-9f32, b);
    *red = add(t3, add(t1, t2));

    // (t2 + t1) - t3
    let t1 = mul(r, 0.17055699092998f32);
    let t2 = mul(0.82944300785005f32, g);
    let t3 = mul(5.98679E-10f32, b);
    *green = sub(add(t2, t1), t3);

    // (t2 + t1) + B
    let t1 = mul(r, -0.00451714424166f32);
    let t2 = mul(0.00451714427397f32, g);
    *blue = add(add(t2, t1), b);
}

/// ```c
/// static void Deuteranopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = 0.33066007266046f * R + 0.66933992517563f * G + 3.559314E-9f * B;
///     *Green = 0.33066007387760f * R + 0.66933992719147f * G - 1.758327E-9f * B;
///     *Blue = -0.02785538261323f * R + 0.02785538252318f * G + B;
/// }
/// ```
unsafe fn deuteranopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = (*red, *green, *blue);

    // t3 + (t1 + t2)
    let t1 = mul(r, 0.33066007266046f32);
    let t2 = mul(0.66933992517563f32, g);
    let t3 = mul(3.559314E-9f32, b);
    *red = add(t3, add(t1, t2));

    // (t2 + t1) - t3
    let t1 = mul(r, 0.33066007387760f32);
    let t2 = mul(0.66933992719147f32, g);
    let t3 = mul(1.758327E-9f32, b);
    *green = sub(add(t2, t1), t3);

    // (t2 + t1) + B
    let t1 = mul(r, -0.02785538261323f32);
    let t2 = mul(0.02785538252318f32, g);
    *blue = add(add(t2, t1), b);
}

/// ```c
/// static void Tritanopia(float *Red, float *Green, float *Blue) {
///     float R = *Red, G = *Green, B = *Blue;
///     *Red = R + 0.12739886310880f * G - 0.12739886341072f * B;
///     *Green = -4.486E-11f * R + 0.87390929928361f * G + 0.12609070101523f * B;
///     *Blue = 3.1113E-10f * R + 0.87390929725848f * G + 0.12609070067115f * B;
/// }
/// ```
unsafe fn tritanopia(red: *mut f32, green: *mut f32, blue: *mut f32) {
    let (r, g, b) = (*red, *green, *blue);

    // (tG + R) - tB
    let t_g = mul(0.12739886310880f32, g);
    let t_b = mul(0.12739886341072f32, b);
    *red = sub(add(t_g, r), t_b);

    // t3 + (t1 + t2)
    let t1 = mul(r, -4.486E-11f32);
    let t2 = mul(0.87390929928361f32, g);
    let t3 = mul(0.12609070101523f32, b);
    *green = add(t3, add(t1, t2));

    // t3 + (t1 + t2)
    let t1 = mul(r, 3.1113E-10f32);
    let t2 = mul(0.87390929725848f32, g);
    let t3 = mul(0.12609070067115f32, b);
    *blue = add(t3, add(t1, t2));
}

/// `void colourblind(cb_impairment Impairment, float *R, float *G, float *B);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn colourblind(
    impairment: cb_impairment,
    r: *mut f32,
    g: *mut f32,
    b: *mut f32,
) {
    match impairment {
        cbProtanopia => protanopia(r, g, b),
        cbDeuteranopia => deuteranopia(r, g, b),
        cbTritanopia => tritanopia(r, g, b),
        // No `default` label in the C switch: every other value does nothing.
        _ => {}
    }
}
