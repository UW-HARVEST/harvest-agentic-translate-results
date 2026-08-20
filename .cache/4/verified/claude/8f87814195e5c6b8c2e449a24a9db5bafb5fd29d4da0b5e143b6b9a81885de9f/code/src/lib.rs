//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! Source: `c_src/src/lib.c`, header: `c_src/include/lib.h`
//!
//! # Bit-exactness notes
//!
//! The C code is compiled for x86-64 with SSE scalar arithmetic. Two families
//! of behaviour are *not* expressible with plain Rust `f32` operators, because
//! LLVM treats all NaNs as interchangeable and Rust's `as` casts saturate:
//!
//! 1. **NaN payload propagation order.** `MULSS`/`SUBSS`/`DIVSS` return the
//!    *first* source operand (quieted) when it is a NaN, otherwise the second
//!    source operand (quieted) when *that* is a NaN, otherwise — if the
//!    operation is invalid, e.g. `0 * inf` or `inf - inf` — the x86
//!    "QNaN floating-point indefinite", `0xFFC0_0000`. Plain Rust `*`/`-`
//!    lower to commuted instruction orders depending on the optimisation
//!    level, which changes which payload survives. [`mulss`], [`subss`] and
//!    [`divss`] reproduce the exact operand order that GCC emits for
//!    `c_src/src/lib.c` (verified against its disassembly).
//! 2. **Out-of-range `float` -> `int` conversion.** `(int)floorf(h)` is UB in C;
//!    on x86-64 `CVTTSS2SI` yields the integer-indefinite value `INT_MIN`
//!    for NaN, the infinities, and anything outside `[-2^31, 2^31)`. See
//!    [`cvttss2si`].
//!
//! All values are moved through their raw `u32` bit patterns so that signaling
//! NaNs pass through the copy paths untouched, exactly as the C `movss` stores
//! do.
//!
//! Memory is accessed with [`std::ptr::read`]/[`std::ptr::write`] rather than
//! the `*ptr` deref operator on purpose: the deref operator makes rustc emit
//! debug-profile `null_pointer_dereference` / `misaligned_pointer_dereference`
//! UB assertions, which abort with a Rust diagnostic where the C code simply
//! faults (or, for misalignment, simply works). Using the `ptr::` accessors
//! keeps the debug and release objects behaviourally identical to the C one for
//! those out-of-contract pointers too.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;

/// x86 "QNaN floating-point indefinite", produced by an SSE invalid operation
/// when neither source operand is itself a NaN (e.g. `0 * inf`, `inf - inf`).
const INDEFINITE: u32 = 0xFFC0_0000;

/// The mantissa MSB; setting it turns a signaling NaN into the corresponding
/// quiet NaN while preserving sign and payload (what x86 calls "quieting").
const QUIET_BIT: u32 = 0x0040_0000;

const EXP_MASK: u32 = 0x7F80_0000;
const MANT_MASK: u32 = 0x007F_FFFF;

#[inline]
fn is_nan_bits(b: u32) -> bool {
    (b & EXP_MASK) == EXP_MASK && (b & MANT_MASK) != 0
}

/// Shared NaN-propagation prologue of the SSE scalar arithmetic instructions:
/// `SRC1` wins over `SRC2`, and the surviving NaN is quieted.
#[inline]
fn sse_nan_result(src1: f32, src2: f32) -> Option<f32> {
    let b1 = src1.to_bits();
    if is_nan_bits(b1) {
        return Some(f32::from_bits(b1 | QUIET_BIT));
    }
    let b2 = src2.to_bits();
    if is_nan_bits(b2) {
        return Some(f32::from_bits(b2 | QUIET_BIT));
    }
    None
}

/// `SUBSS src1, src2` -> `src1 - src2`.
#[inline]
fn subss(src1: f32, src2: f32) -> f32 {
    if let Some(n) = sse_nan_result(src1, src2) {
        return n;
    }
    let r = src1 - src2;
    if r.is_nan() {
        // Invalid operation with non-NaN operands (`inf - inf`).
        f32::from_bits(INDEFINITE)
    } else {
        r
    }
}

/// `MULSS src1, src2` -> `src1 * src2`.
#[inline]
fn mulss(src1: f32, src2: f32) -> f32 {
    if let Some(n) = sse_nan_result(src1, src2) {
        return n;
    }
    let r = src1 * src2;
    if r.is_nan() {
        // Invalid operation with non-NaN operands (`0 * inf`).
        f32::from_bits(INDEFINITE)
    } else {
        r
    }
}

/// `DIVSS src1, src2` -> `src1 / src2`.
#[inline]
fn divss(src1: f32, src2: f32) -> f32 {
    if let Some(n) = sse_nan_result(src1, src2) {
        return n;
    }
    let r = src1 / src2;
    if r.is_nan() {
        // Invalid operation with non-NaN operands (`0/0`, `inf/inf`).
        f32::from_bits(INDEFINITE)
    } else {
        r
    }
}

/// `floorf` from `<math.h>`, bit-exact with glibc on x86-64.
///
/// glibc's `floorf` is `roundToIntegralTowardNegative`, which `f32::floor`
/// implements for every finite/infinite input (including preserving `-0.0`).
/// For a NaN argument glibc computes `x + x` (generic implementation) or
/// `ROUNDSS` (the SSE4.1 multiarch variant); both return the argument
/// *quieted*, preserving sign and payload, which is what we do explicitly here
/// rather than relying on LLVM's NaN-agnostic lowering.
#[inline]
fn floorf(x: f32) -> f32 {
    let b = x.to_bits();
    if is_nan_bits(b) {
        return f32::from_bits(b | QUIET_BIT);
    }
    x.floor()
}

/// Reproduce the x86-64 `CVTTSS2SI` semantics that GCC/Clang emit for the C
/// expression `(int)some_float`.
///
/// In C, converting a `float` whose truncated value is not representable in
/// `int` (including NaN and the infinities) is undefined behaviour. On x86-64
/// the hardware conversion instruction yields the "integer indefinite" value
/// `0x80000000` (`INT_MIN`) in those cases. Rust's `as` cast instead saturates
/// (NaN maps to 0, out-of-range maps to `i32::MIN`/`i32::MAX`), which would
/// select a *different* `switch` arm than the C code for NaN and for large
/// positive hues. We therefore emulate the C/hardware behaviour so the output
/// stays byte-identical.
#[inline]
fn cvttss2si(x: f32) -> c_int {
    // NaN, +/-inf and anything outside [-2^31, 2^31) => integer indefinite.
    if x.is_nan() || !(x >= -2147483648.0f32 && x < 2147483648.0f32) {
        c_int::MIN
    } else {
        x as c_int
    }
}

/// Convert an HSV triple to an RGB triple.
///
/// `src` points to at least 3 `float`s: hue (in degrees), saturation, value.
/// `dest` points to at least 3 writable `float`s that receive red, green, blue.
///
/// Mirrors `c_src/src/lib.c` statement for statement, including reading all
/// three `src` elements *before* any store (so aliasing `dest` with `src` is
/// well defined, just as in C) and writing *only* `dest[0..3]`.
///
/// # Safety
///
/// `src` must be valid for reads of 3 `f32`s and `dest` valid for writes of
/// 3 `f32`s, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    // Raw bit patterns, so signaling NaNs survive the copy paths verbatim.
    let src_bits = src as *const u32;
    let dest_bits = dest as *mut u32;

    let h_bits: u32 = unsafe { src_bits.add(0).read() };
    let s_bits: u32 = unsafe { src_bits.add(1).read() };
    let v_bits: u32 = unsafe { src_bits.add(2).read() };

    let mut h: f32 = f32::from_bits(h_bits);
    let s: f32 = f32::from_bits(s_bits);
    let v: f32 = f32::from_bits(v_bits);

    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;
    let i: c_int;

    // `if (s == 0)` -- true for both +0.0 and -0.0, false for NaN.
    if s == 0.0f32 {
        unsafe {
            dest_bits.add(0).write(v_bits);
            dest_bits.add(1).write(v_bits);
            dest_bits.add(2).write(v_bits);
        }
        return;
    }

    // h /= 60.0f;            DIVSS h, 60.0
    h = divss(h, 60.0f32);
    // i = (int)floorf(h);    CVTTSS2SI
    i = cvttss2si(floorf(h));
    // f = h - i;             SUBSS h, (float)i
    f = subss(h, i as f32);
    // p = v * (1 - s);       SUBSS 1.0, s   then  MULSS (1-s), v
    p = mulss(subss(1.0f32, s), v);
    // q = v * (1 - s * f);   MULSS s, f  ->  SUBSS 1.0, .  ->  MULSS ., v
    q = mulss(subss(1.0f32, mulss(s, f)), v);
    // t = v * (1 - s * (1 - f));
    //     SUBSS 1.0, f  ->  MULSS (1-f), s  ->  SUBSS 1.0, .  ->  MULSS ., v
    t = mulss(subss(1.0f32, mulss(subss(1.0f32, f), s)), v);

    // GCC lowers the `switch` to an unsigned `cmpl $4 / ja`, so every negative
    // `i` (and `INT_MIN`) also lands in `default:`.
    match i {
        0 => {
            r = v;
            g = t;
            b = p;
        }
        1 => {
            r = q;
            g = v;
            b = p;
        }
        2 => {
            r = p;
            g = v;
            b = t;
        }
        3 => {
            r = p;
            g = q;
            b = v;
        }
        4 => {
            r = t;
            g = p;
            b = v;
        }
        _ => {
            r = v;
            g = p;
            b = q;
        }
    }

    unsafe {
        dest_bits.add(0).write(r.to_bits());
        dest_bits.add(1).write(g.to_bits());
        dest_bits.add(2).write(b.to_bits());
    }
}
