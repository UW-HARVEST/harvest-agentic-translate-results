//! Translation of `c_src/src/spectral_contrast.c`.
//!
//! This translation unit does **not** include `match.h`, so its `float_t` is
//! `<math.h>`'s C99 `float_t`, i.e. `float` on x86-64 Linux. See the crate
//! docs.

use core::ffi::{c_double, c_float, c_int};

use crate::clamp_len;
use crate::sse::{addsd, divsd, mulss};

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
/// `cvtss2sd`, `addsd`). The `f32` round-trip below is load-bearing.
///
/// The emitted `mulss` has `b[i]` as its destination and `a[i]` as its source,
/// and the `addsd` has the widened product as destination and the accumulator as
/// source; [`mulss`] and [`addsd`] reproduce that so NaN payloads agree.
///
/// # Safety
/// `a` and `b` must be valid for reads of `len` `f32`s. They may alias, so raw
/// pointers are used rather than slices.
unsafe fn dot_product(a: *const f32, b: *const f32, len: usize) -> f64 {
    let mut sum: f64 = 0.0;
    for i in 0..len {
        let (ai, bi) = unsafe { (*a.add(i), *b.add(i)) };
        let product: f32 = mulss(bi, ai);
        sum = addsd(product as f64, sum);
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
/// `v[i] /= magnitude` widens the element to `double`, divides in `double`, then
/// narrows back to `float`. A zero or NaN magnitude is not guarded against in
/// the C, and is not guarded against here.
///
/// `dot_product(v, v, ...)` sums squares, so its result is never negative;
/// `sqrt` therefore only ever sees a non-negative value, `+inf`, or a NaN, and
/// `sqrtsd` propagates the latter unchanged apart from quieting.
///
/// # Safety
/// `v` must be valid for reads and writes of `len` `f32`s.
unsafe fn normalize(v: *mut f32, len: usize) {
    let magnitude: f64 = unsafe { dot_product(v, v, len) }.sqrt();
    for i in 0..len {
        let element = unsafe { *v.add(i) };
        unsafe { *v.add(i) = divsd(element as f64, magnitude) as f32 };
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
/// `a` and `b` must each be valid for reads and writes of `length` `float`s.
/// They are allowed to alias, exactly as in the C. Callers of the C library that
/// follow `match.h` will pass `double` arrays instead; only the leading
/// `length * 4` bytes are touched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(
    a: *mut c_float,
    b: *mut c_float,
    length: c_int,
) -> c_double {
    let len = clamp_len(length);
    unsafe {
        normalize(a, len);
        normalize(b, len);
        dot_product(a, b, len)
    }
}
