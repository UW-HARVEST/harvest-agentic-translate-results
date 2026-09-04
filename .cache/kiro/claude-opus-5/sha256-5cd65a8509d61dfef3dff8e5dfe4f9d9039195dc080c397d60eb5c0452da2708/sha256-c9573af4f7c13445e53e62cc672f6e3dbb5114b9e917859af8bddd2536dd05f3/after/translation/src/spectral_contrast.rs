//! Rust translation of `c_src/src/spectral_contrast.c`.
//!
//! ## The `float_t` trap (reproduced, not fixed)
//!
//! `c_src/src/spectral_contrast.c` includes **only** `<math.h>` -- it never
//! includes `match.h`. Therefore the `float_t` it uses is *not* the
//! `typedef double float_t;` from `match.h`, but the C99 `float_t` from
//! `<math.h>`. On x86-64 glibc `__FLT_EVAL_METHOD__ == 0`, so
//! `float_t` is `float` (4 bytes).
//!
//! Confirmed against the compiled C shared object: `dot_product` / `normalize`
//! use `movss` / `mulss` and a 4-byte element stride
//! (`lea 0x0(,%rax,4),%rdx`), i.e. they walk their arguments as `float *`,
//! while `match` (which sees `match.h`) walks its arrays as `double *`.
//!
//! This is a bug in the original C, and per the translation contract it is
//! reproduced exactly: `spectral_contrast` operates on `f32` elements.
//!
//! ## Aliasing
//!
//! `include/match.h` has no `restrict`, so `spectral_contrast(a, a, n)` is a
//! legal call that normalises one buffer twice. Everything below therefore
//! works on raw pointers rather than `&mut [f32]`: two overlapping `&mut`
//! slices would be instant Rust UB and LLVM's `noalias` could reorder the
//! `normalize` passes.

use std::ffi::c_int;

use crate::fp::{add_sd, cvtsd2ss, cvtss2sd, mul_ss};

/// `static double dot_product(float_t *a, float_t *b, int length)`
///
/// `a[i] * b[i]` is a `float * float` product. With `FLT_EVAL_METHOD == 0` the
/// multiply happens in single precision (`mulss`), and only the *result* is
/// widened to `double` before being accumulated (`cvtss2sd` + `addsd`).
///
/// GCC at `-O0` emits, per iteration:
/// ```text
///   movss  (a+4i),%xmm1        ; a[i]  -- loaded first
///   movss  (b+4i),%xmm0        ; b[i]  -- loaded second
///   mulss  %xmm1,%xmm0         ; dst = b[i], src = a[i]
///   cvtss2sd %xmm0,%xmm0
///   movsd  sum,%xmm1
///   addsd  %xmm1,%xmm0         ; dst = product, src = sum
///   movsd  %xmm0,sum
/// ```
/// so `b[i]` is the multiply's destination and the *product* is the add's
/// destination. See `crate::fp` for why those roles must be pinned.
///
/// # Safety
/// `a` and `b` must each be valid for `length` `f32` reads. They may alias.
unsafe fn dot_product(a: *const f32, b: *const f32, length: usize) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..length {
        let ai = unsafe { *a.add(i) };
        let bi = unsafe { *b.add(i) };
        sum = add_sd(cvtss2sd(mul_ss(bi, ai)), sum);
    }
    sum
}

/// `static void normalize(float_t *v, int length)`
///
/// `magnitude` comes from `sqrt(dot_product(v, v, length))`; the C calls libm's
/// `sqrt` through the PLT, which on x86-64 is `sqrtsd`. `dot_product(v, v)` is
/// a sum of squares, so it is never negative -- only `+0.0`, positive, `+inf`
/// or `NaN` reach `sqrt`, and for all of those `sqrtsd` and `f64::sqrt` agree
/// bit-for-bit (including the NaN payload).
///
/// `v[i] /= magnitude` where `v[i]` is `float` and `magnitude` is `double`:
/// widen, divide in double precision, then narrow back to `float`
/// (`cvtss2sd` / `divsd` / `cvtsd2ss`). There is no divide-by-zero guard, so an
/// all-zero vector yields `0.0/0.0` in every lane.
///
/// # Safety
/// `v` must be valid for `length` `f32` reads and writes.
unsafe fn normalize(v: *mut f32, length: usize) {
    let magnitude = unsafe { dot_product(v, v, length) }.sqrt();
    for i in 0..length {
        let x = unsafe { *v.add(i) };
        unsafe { *v.add(i) = cvtsd2ss(cvtss2sd(x) / magnitude) };
    }
}

/// Internal entry point taking raw pointers, so `match` can reach the same code
/// path the C `match` reaches through the PLT.
///
/// # Safety
/// `a` and `b` must each be valid for `length` `f32` reads and writes. They may
/// alias.
pub(crate) unsafe fn spectral_contrast_raw(a: *mut f32, b: *mut f32, length: usize) -> f64 {
    unsafe {
        normalize(a, length);
        normalize(b, length);
        dot_product(a, b, length)
    }
}

/// `double spectral_contrast(float_t *a, float_t *b, int length)`
///
/// Public ABI symbol. Note the element type is `f32` (see module docs).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
    // Every loop in this translation unit is `for(i = 0; i < length; i++)`, so
    // a non-positive `length` degenerates to zero iterations:
    //   dot_product -> +0.0, sqrt(+0.0) -> +0.0, normalize -> no-op, result
    //   +0.0 -- and the pointers are never dereferenced, so even NULL is fine.
    if length <= 0 {
        return 0.0;
    }
    unsafe { spectral_contrast_raw(a, b, length as usize) }
}
