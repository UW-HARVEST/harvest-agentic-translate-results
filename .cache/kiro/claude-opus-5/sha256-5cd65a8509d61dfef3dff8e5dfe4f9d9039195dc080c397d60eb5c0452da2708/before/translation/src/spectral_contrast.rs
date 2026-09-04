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
//! `movss` / `mulss` and a 4-byte element stride, i.e. it walks its arguments
//! as `float *`, while `match` (which sees `match.h`) walks its arrays as
//! `double *`.
//!
//! This is a bug in the original C, and per the translation contract it is
//! reproduced exactly: `spectral_contrast` operates on `f32` elements.

use std::ffi::c_int;
use std::slice;

use crate::fp::{add_sd, mul_ss};

/// `static double dot_product(float_t *a, float_t *b, int length)`
///
/// `a[i] * b[i]` is a `float * float` product. With `FLT_EVAL_METHOD == 0` the
/// multiply happens in single precision (`mulss`), and only the *result* is
/// widened to `double` before being accumulated (`cvtss2sd` + `addsd`).
///
/// GCC emits `movss a[i]` / `mulss b[i]` / `cvtss2sd` / `addsd sum`, i.e. the
/// multiply's destination is `a[i]` and the add's destination is `sum`; see
/// `crate::fp` for why those roles are pinned explicitly.
fn dot_product(a: &[f32], b: &[f32]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..a.len() {
        sum = add_sd(sum, mul_ss(a[i], b[i]) as f64);
    }
    sum
}

/// `static void normalize(float_t *v, int length)`
///
/// `v[i] /= magnitude` where `v[i]` is `float` and `magnitude` is `double`:
/// widen, divide in double precision, then truncate back to `float`
/// (`cvtss2sd` / `divsd` / `cvtsd2ss`).
fn normalize(v: &mut [f32]) {
    let magnitude = dot_product(v, v).sqrt();
    for i in 0..v.len() {
        v[i] = ((v[i] as f64) / magnitude) as f32;
    }
}

/// Internal entry point operating on already-checked slices, so that `match`
/// can reach the same code path the C `match` reaches through the PLT.
pub(crate) fn spectral_contrast_slices(a: &mut [f32], b: &mut [f32]) -> f64 {
    normalize(a);
    normalize(b);
    dot_product(a, b)
}

/// `double spectral_contrast(float_t *a, float_t *b, int length)`
///
/// Public ABI symbol. Note the element type is `f32` (see module docs).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
    // Every loop in this translation unit is `for(i = 0; i < length; i++)`, so
    // a non-positive `length` degenerates to zero iterations:
    //   dot_product -> 0.0, sqrt(0.0) -> 0.0, normalize -> no-op, result 0.0.
    if length <= 0 {
        return 0.0;
    }
    let n = length as usize;
    let a = unsafe { slice::from_raw_parts_mut(a, n) };
    let b = unsafe { slice::from_raw_parts_mut(b, n) };
    spectral_contrast_slices(a, b)
}
