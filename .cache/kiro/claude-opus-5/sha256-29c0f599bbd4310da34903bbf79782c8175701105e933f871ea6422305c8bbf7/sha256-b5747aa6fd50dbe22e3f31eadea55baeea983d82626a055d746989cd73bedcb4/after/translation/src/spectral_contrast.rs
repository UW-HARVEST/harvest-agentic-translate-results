//! Translation of `c_src/src/spectral_contrast.c`.
//!
//! This translation unit does **not** include `match.h`, so its `float_t` is
//! `<math.h>`'s C99 `float_t`, i.e. `float` on x86-64 Linux. See the crate
//! docs.

use core::ffi::{c_double, c_float, c_int};

use crate::{clamp_len, slice_from_raw_mut};

/// ```c
/// static double dot_product(float_t *a, float_t *b, int length) {
///     double sum = 0;
///     int i;
///     for(i = 0; i < length; i++) sum += a[i] * b[i];
///     return sum;
/// }
/// ```
///
/// `a[i] * b[i]` multiplies two `float`s, so the product is rounded to `float`
/// *before* being widened and accumulated into the `double` sum (`mulss`,
/// `cvtss2sd`, `addsd`). The `as f32` round-trip below is load-bearing.
fn dot_product(a: &[f32], b: &[f32]) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..a.len() {
        let product: f32 = a[i] * b[i];
        sum += product as f64;
    }
    sum
}

/// ```c
/// static void normalize(float_t *v, int length) {
///     double magnitude = sqrt(dot_product(v, v, length));
///     int i;
///     for(i = 0; i < length; i++) v[i] /= magnitude;
/// }
/// ```
///
/// `v[i] /= magnitude` widens the element to `double`, divides in `double`,
/// then narrows back to `float`. A zero or NaN magnitude is not guarded against
/// in the C, and is not guarded against here.
fn normalize(v: &mut [f32]) {
    let magnitude: f64 = dot_product(&*v, &*v).sqrt();
    for element in v.iter_mut() {
        *element = (*element as f64 / magnitude) as f32;
    }
}

/// ```c
/// double spectral_contrast(float_t *a, float_t *b, int length) {
///     normalize(a, length);
///     normalize(b, length);
///     return dot_product(a, b, length);
/// }
/// ```
///
/// # Safety
/// `a` and `b` must each be valid for reads and writes of `length` `float`s and
/// must not alias. Callers of the C library that follow `match.h` will pass
/// `double` arrays instead; only the leading `length * 4` bytes are touched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut c_float,
    b: *mut c_float,
    length: c_int,
) -> c_double {
    let len = clamp_len(length);
    let a = unsafe { slice_from_raw_mut(a, len) };
    let b = unsafe { slice_from_raw_mut(b, len) };
    normalize(a);
    normalize(b);
    dot_product(a, b)
}
