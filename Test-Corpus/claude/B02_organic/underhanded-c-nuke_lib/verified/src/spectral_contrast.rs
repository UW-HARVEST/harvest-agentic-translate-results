// Translation of c_src/src/spectral_contrast.c
//
//     #include <math.h> /* sqrt */
//
//     static double dot_product(float_t *a, float_t *b, int length) { ... }
//     static void normalize(float_t *v, int length) { ... }
//     double spectral_contrast(float_t *a, float_t *b, int length) { ... }
//
// This translation unit never includes "match.h", so `float_t` is the one from
// <math.h>: on x86-64 glibc (FLT_EVAL_METHOD == 0) that is plain `float`.
// Hence every `float_t` below is `f32`.

use core::ffi::c_int;

/// static double dot_product(float_t *a, float_t *b, int length)
///
/// ```c
/// double sum = 0;
/// int i;
/// for(i = 0; i < length; i++) sum += a[i] * b[i];
/// return sum;
/// ```
///
/// `a[i] * b[i]` multiplies two `float`s, producing a `float`; the `float`
/// result is then converted to `double` and accumulated (gcc: mulss, cvtss2sd,
/// addsd).  The intermediate rounding to single precision is deliberate.
///
/// gcc emits, per iteration:
///
/// ```text
///     movss  (a+i), %xmm1        ; SRC2
///     movss  (b+i), %xmm0        ; SRC1 / dest
///     mulss  %xmm1, %xmm0        ; xmm0 = b[i] * a[i]
///     cvtss2sd %xmm0, %xmm0
///     movsd  sum, %xmm1          ; SRC2
///     addsd  %xmm1, %xmm0        ; xmm0 = product + sum
/// ```
///
/// so `b[i]` is the multiply's SRC1 and the freshly computed `product` is the
/// add's SRC1.  That ordering is invisible for finite values but decides which
/// NaN payload survives, hence `fp::mulss` / `fp::addsd`.
#[inline]
unsafe fn dot_product(a: *const f32, b: *const f32, length: c_int) -> f64 {
    let mut sum: f64 = 0.0;
    let mut i: c_int = 0;
    while i < length {
        let av: f32 = unsafe { *a.offset(i as isize) };
        let bv: f32 = unsafe { *b.offset(i as isize) };
        let product: f32 = crate::fp::mulss(bv, av);
        sum = crate::fp::addsd(product as f64, sum);
        i = i.wrapping_add(1);
    }
    sum
}

/// static void normalize(float_t *v, int length)
///
/// ```c
/// double magnitude = sqrt(dot_product(v, v, length));
/// int i;
/// for(i = 0; i < length; i++) v[i] /= magnitude;
/// ```
///
/// `v[i] /= magnitude` is `v[i] = (float)((double)v[i] / magnitude)`.
#[inline]
unsafe fn normalize(v: *mut f32, length: c_int) {
    let magnitude: f64 = unsafe { dot_product(v, v, length) }.sqrt();
    let mut i: c_int = 0;
    while i < length {
        let p = unsafe { v.offset(i as isize) };
        let cur: f32 = unsafe { *p };
        let scaled: f64 = (cur as f64) / magnitude;
        unsafe { *p = scaled as f32 };
        i = i.wrapping_add(1);
    }
}

/// double spectral_contrast(float_t *a, float_t *b, int length)
///
/// ```c
/// normalize(a, length);
/// normalize(b, length);
/// return dot_product(a, b, length);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn spectral_contrast(a: *mut f32, b: *mut f32, length: c_int) -> f64 {
    unsafe {
        normalize(a, length);
        normalize(b, length);
        dot_product(a, b, length)
    }
}
