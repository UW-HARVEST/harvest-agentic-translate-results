//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (verified with `nm -D` against the C shared library):
//!   * `tfm`   —  `void tfm(float *dest, const float *src, int count)`
//!
//! ## Bit-exactness notes
//!
//! The C library is a single `float` kernel, so every finite result is fixed by
//! IEEE-754 single precision and a straight operation-for-operation port is
//! already bit-exact. The one place where a naive port diverges is *which* NaN
//! comes out of an operation that has a NaN operand.
//!
//! On x86-64 SSE, `ADDSS/SUBSS/MULSS dest, src` returns the quieted `dest`
//! operand when `dest` is a NaN, and otherwise the quieted `src` operand when
//! `src` is a NaN. So the payload and sign bit of the result depend on which
//! side of the C expression the compiler parked in the destination register.
//! Reassociating the operands - even in a way that is value-identical for
//! non-NaN inputs - changes the observable output bits.
//!
//! The reference `.so` is compiled at `-O0`, and its `tfm` schedules the
//! `sqd` expression as (see `objdump -d`):
//!
//! ```text
//!   xmm1 = dy2 * dy2                  mulss  dest=dy2      src=dy2
//!   xmm0 = dx2 + dx2                  addss  dest=dx2      src=dx2   (== 2.0f*dx2)
//!   xmm0 = xmm0 * dy2                 mulss  dest=2dx2     src=dy2
//!   xmm1 = xmm1 - xmm0                subss  dest=dy2*dy2  src=2dx2*dy2
//!   xmm0 = dx2 * dx2                  mulss  dest=dx2      src=dx2
//!   xmm1 = xmm1 + xmm0                addss  dest=acc      src=dx2*dx2
//!   xmm0 = 4.0f * dxy                 mulss  dest=4.0f     src=dxy
//!   xmm0 = xmm0 * dxy                 mulss  dest=4dxy     src=dxy
//!   xmm0 = xmm0 + xmm1                addss  dest=4dxy*dxy src=acc   <-- reversed
//! ```
//!
//! Note the final accumulation puts the `4*dxy*dxy` term in the *destination*,
//! which is why a NaN `dxy` wins over a NaN accumulator there. The helpers
//! below model that instruction-level behaviour explicitly, so the exact
//! operand roles are reproduced rather than left to LLVM's register allocator.
//! For non-NaN operands the helpers fall through to the native `f32` op, which
//! is the same single instruction, including the x86 "indefinite" QNaN
//! (`0xffc00000`) produced by invalid operations such as `0 * inf`.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// x86-64 SSE scalar single-precision primitives
// ---------------------------------------------------------------------------

/// Quiet a NaN the way SSE does: set the quiet bit, keep the sign and payload.
#[inline(always)]
fn quiet(x: f32) -> f32 {
    f32::from_bits(x.to_bits() | 0x0040_0000)
}

/// `ADDSS dest, src`
#[inline(always)]
fn addss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest + src
    }
}

/// `SUBSS dest, src`
#[inline(always)]
fn subss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest - src
    }
}

/// `MULSS dest, src`
#[inline(always)]
fn mulss(dest: f32, src: f32) -> f32 {
    if dest.is_nan() {
        quiet(dest)
    } else if src.is_nan() {
        quiet(src)
    } else {
        dest * src
    }
}

/// `sqrtf` from `<math.h>`.
///
/// glibc's wrapper is `isless(x, 0) ? __math_invalidf(x) : __ieee754_sqrtf(x)`,
/// and `__ieee754_sqrtf` is a bare `SQRTSS`. `isless` is false for NaN, so a
/// NaN argument is passed through `SQRTSS`, which quiets it. Negative arguments
/// are unreachable here because the caller clamps at zero first, and both
/// glibc's `__math_invalidf` and `SQRTSS` yield `0xffc00000` for them anyway.
#[inline(always)]
fn sqrtf(x: f32) -> f32 {
    if x.is_nan() {
        quiet(x)
    } else {
        x.sqrt()
    }
}

// ---------------------------------------------------------------------------
// Kernel
// ---------------------------------------------------------------------------

/// The body shared by both arms of the `src[0] < src[1]` test.
///
/// Both arms in the C source compute the identical `sqd`/`lambda` sequence;
/// they differ only in which of `src[0]`/`src[1]` is named `dx2` vs `dy2`, and
/// in the order the two results are stored.
#[inline(always)]
fn lambda_of(dx2: f32, dy2: f32, dxy: f32) -> f32 {
    // sqd = (dy2*dy2) - (2.0f*dx2*dy2) + (dx2*dx2) + (4.0f*dxy*dxy)
    let mut acc = mulss(dy2, dy2);
    let two_dx2 = addss(dx2, dx2); // gcc strength-reduces 2.0f*dx2 to dx2+dx2
    acc = subss(acc, mulss(two_dx2, dy2));
    acc = addss(acc, mulss(dx2, dx2));
    let sqd = addss(mulss(mulss(4.0f32, dxy), dxy), acc);

    // ((0) > (sqd)) ? (0) : (sqd)
    //
    // The `0` is an `int` promoted to `float`, so this is a clamp at +0.0f.
    // COMISS reports "unordered" for a NaN `sqd`, which fails the `>` test, so
    // a NaN falls through to `sqd` instead of being clamped away.
    let clamped = if 0.0f32 > sqd { 0.0f32 } else { sqd };

    // lambda = 0.5f * (dy2 + dx2 + sqrtf(clamped))
    mulss(0.5f32, addss(addss(dy2, dx2), sqrtf(clamped)))
}

/// `void tfm(float *dest, const float *src, int count);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(mut dest: *mut f32, mut src: *const f32, count: c_int) {
    let mut i: c_int = 0;
    while i < count {
        let s0 = *src;
        let s1 = *src.add(1);
        let s2 = *src.add(2);

        if s0 < s1 {
            let (dx2, dy2, dxy) = (s0, s1, s2);
            let lambda = lambda_of(dx2, dy2, dxy);
            *dest = subss(dx2, lambda);
            *dest.add(1) = dxy;
        } else {
            let (dy2, dx2, dxy) = (s0, s1, s2);
            let lambda = lambda_of(dx2, dy2, dxy);
            *dest = dxy;
            *dest.add(1) = subss(dx2, lambda);
        }

        src = src.add(3);
        dest = dest.add(2);

        i += 1;
    }
}
