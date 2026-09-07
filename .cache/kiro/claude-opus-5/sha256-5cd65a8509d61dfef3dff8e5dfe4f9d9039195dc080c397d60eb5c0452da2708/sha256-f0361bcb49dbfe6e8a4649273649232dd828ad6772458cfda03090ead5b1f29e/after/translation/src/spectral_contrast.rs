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
//! Confirmed against the compiled C shared object: `spectral_contrast` uses
//! `movss` / `mulss` / `cvtss2sd` / `cvtsd2ss` and a 4-byte element stride, i.e.
//! it walks its arguments as `float *`, while `match` (which sees `match.h`)
//! walks its arrays as `double *`.
//!
//! This is a bug in the original C, and per the translation contract it is
//! reproduced exactly: `spectral_contrast` operates on `f32` elements.
//!
//! ## Why raw pointers instead of slices
//!
//! Every loop here is `for(i = 0; i < length; i++)`, so a non-positive `length`
//! performs **zero** iterations and never dereferences its arguments -- the C
//! accepts `spectral_contrast(NULL, NULL, 0)` and returns `+0.0`. Conversely a
//! positive `length` with a `NULL` pointer must fault exactly where the C faults.
//! Building a `&[f32]` up front would do neither: it would trip Rust's
//! `slice::from_raw_parts` null/alignment precondition (an abort, not a
//! segfault, and only in debug builds). Walking raw pointers reproduces both
//! behaviours in every build profile.

use std::ffi::c_int;

use crate::fp::{add_sd, mul_ss};

/// `static double dot_product(float_t *a, float_t *b, int length)`
///
/// `a[i] * b[i]` is a `float * float` product. With `FLT_EVAL_METHOD == 0` the
/// multiply happens in single precision (`mulss`), and only the *result* is
/// widened to `double` before being accumulated (`cvtss2sd` + `addsd`).
///
/// GCC emits, per iteration:
///
/// ```text
/// movss    a[i], %xmm1        ; xmm1 = a[i]
/// movss    b[i], %xmm0        ; xmm0 = b[i]
/// mulss    %xmm1, %xmm0       ; DEST = b[i], SRC = a[i]
/// cvtss2sd %xmm0, %xmm0
/// movsd    sum,  %xmm1
/// addsd    %xmm1, %xmm0       ; DEST = product, SRC = sum
/// movsd    %xmm0, sum
/// ```
///
/// so the multiply's destination is `b[i]` (not `a[i]`) and the add's
/// destination is the *product* (not the accumulator). Both roles are pinned
/// explicitly because SSE resolves a two-NaN operand pair in favour of the
/// destination; see `crate::fp`.
unsafe fn dot_product(a: *const f32, b: *const f32, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        let av = unsafe { *a.offset(i as isize) };
        let bv = unsafe { *b.offset(i as isize) };
        sum = add_sd(mul_ss(bv, av) as f64, sum);
        i += 1;
    }
    sum
}

/// `static void normalize(float_t *v, int length)`
///
/// `v[i] /= magnitude` where `v[i]` is `float` and `magnitude` is `double`:
/// widen, divide in double precision, then truncate back to `float`
/// (`cvtss2sd` / `divsd` / `cvtsd2ss`). `DIVSD`'s destination is fixed by the
/// operand order, so no helper is needed.
///
/// The C calls glibc `sqrt` through the PLT; on x86-64 that is the `sqrtsd`
/// instruction, which is what `f64::sqrt` lowers to as well. `dot_product(v, v)`
/// is a sum of squares, so it is never negative and only the NaN and `+inf`
/// cases are interesting -- both propagate identically.
unsafe fn normalize(v: *mut f32, length: c_int) {
    let magnitude = unsafe { dot_product(v, v, length) }.sqrt();
    let mut i: c_int = 0;
    while i < length {
        let p = unsafe { v.offset(i as isize) };
        unsafe { *p = ((*p as f64) / magnitude) as f32 };
        i += 1;
    }
}

/// Body of `spectral_contrast`, shared with `match`'s call through the PLT.
pub(crate) unsafe fn spectral_contrast_raw(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
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
    unsafe { spectral_contrast_raw(a, b, length) }
}
