//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (the complete set of symbols exported by the C shared library,
//! per `nm -D`):
//!   * `hsl_to_rgb`   (declared in `c_src/include/lib.h`, defined in
//!                     `c_src/src/lib.c`)
//!
//! The translation is deliberately literal: the operation order, the order of
//! the comparisons, and the quirks of the original are all preserved. In
//! particular the third branch of the dispatch chain tests `h < 120.0f` twice
//! instead of `h >= 120.0f && h < 180.0f`; that is a bug in the C, so it is
//! reproduced here verbatim rather than fixed.
//!
//! To get bit-for-bit identical results even for exotic inputs (infinities and
//! NaNs), the arithmetic goes through small helpers that reproduce the exact
//! NaN-propagation rules of the scalar SSE instructions the C compiler emits
//! (`addss`/`subss`/`mulss`/`divss`): if the first source operand is a NaN the
//! result is that NaN made quiet, otherwise if the second source operand is a
//! NaN the result is that NaN made quiet. Relying on plain `+`/`-`/`*` would
//! leave the choice of operand order (and hence the NaN sign/payload that
//! survives) up to LLVM's canonicalisation.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_float;

unsafe extern "C" {
    /// `float fmodf(float, float)` from libm, exactly as used by the C source.
    safe fn fmodf(x: c_float, y: c_float) -> c_float;
}

/// Set the "quiet" bit of a NaN, as every SSE arithmetic instruction does when
/// it forwards a source NaN to its result.
#[inline]
fn quiet(v: c_float) -> c_float {
    c_float::from_bits(v.to_bits() | 0x0040_0000)
}

/// SSE scalar NaN-operand selection: `src1` wins over `src2`.
#[inline]
fn nan_result(src1: c_float, src2: c_float) -> Option<c_float> {
    if src1.is_nan() {
        Some(quiet(src1))
    } else if src2.is_nan() {
        Some(quiet(src2))
    } else {
        None
    }
}

/// `addss src1, src2`
#[inline]
fn add_ss(src1: c_float, src2: c_float) -> c_float {
    match nan_result(src1, src2) {
        Some(v) => v,
        None => src1 + src2,
    }
}

/// `subss src1, src2`
#[inline]
fn sub_ss(src1: c_float, src2: c_float) -> c_float {
    match nan_result(src1, src2) {
        Some(v) => v,
        None => src1 - src2,
    }
}

/// `mulss src1, src2`
#[inline]
fn mul_ss(src1: c_float, src2: c_float) -> c_float {
    match nan_result(src1, src2) {
        Some(v) => v,
        None => src1 * src2,
    }
}

/// `divss src1, src2`
#[inline]
fn div_ss(src1: c_float, src2: c_float) -> c_float {
    match nan_result(src1, src2) {
        Some(v) => v,
        None => src1 / src2,
    }
}

/// `fabsf` — clears the sign bit (`andps` with `0x7fffffff`), NaNs included.
#[inline]
fn fabsf(v: c_float) -> c_float {
    c_float::from_bits(v.to_bits() & 0x7fff_ffff)
}

/// `void hsl_to_rgb(float *dest, const float *src);`
///
/// `src[0]` is the hue in degrees, `src[1]` the saturation and `src[2]` the
/// lightness; the corresponding red, green and blue components are stored in
/// `dest[0]`, `dest[1]` and `dest[2]`.
///
/// # Safety
///
/// `src` must be valid for reads of three `float`s and `dest` valid for writes
/// of three `float`s — the same contract the C function imposes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    // float h = src[0];
    // float s = src[1];
    // float l = src[2];
    let h: c_float = unsafe { *src.add(0) };
    let s: c_float = unsafe { *src.add(1) };
    let l: c_float = unsafe { *src.add(2) };

    // Stores dest[0..3]; the C writes the three slots in ascending order.
    let store = |r: c_float, g: c_float, b: c_float| unsafe {
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    };

    // if (s == 0) { dest[0] = dest[1] = dest[2] = l; return; }
    // A NaN saturation compares unequal, so it falls through, as in C.
    if s == 0.0 {
        store(l, l, l);
        return;
    }

    // c = (1.0f - fabsf(2.0f * l - 1.0f)) * s;
    let c: c_float = mul_ss(sub_ss(1.0, fabsf(sub_ss(mul_ss(2.0, l), 1.0))), s);
    // m = 1.0f * (l - 0.5f * c);
    // (multiplying by 1.0f cannot change the value, and the subtraction has
    //  already quietened any NaN, so the factor is a no-op)
    let m: c_float = sub_ss(l, mul_ss(0.5, c));
    // x = c * (1.0f - fabsf(fmodf(h / 60.0f, 2) - 1.0f));
    // The C compiler evaluates this product with the `1.0f - ...` term as the
    // first source operand, which is what decides the surviving NaN when both
    // factors are NaN.
    let x: c_float = mul_ss(
        sub_ss(1.0, fabsf(sub_ss(fmodf(div_ss(h, 60.0), 2.0), 1.0))),
        c,
    );

    if h >= 0.0 && h < 60.0 {
        // dest = { c + m, x + m, m }
        store(add_ss(c, m), add_ss(x, m), m);
    } else if h >= 60.0 && h < 120.0 {
        // dest = { x + m, c + m, m }
        store(add_ss(x, m), add_ss(c, m), m);
    } else if h < 120.0 && h < 180.0 {
        // NOTE: the C source really does test `h < 120.0f` twice here instead
        // of `h >= 120.0f && h < 180.0f`. Preserved as-is: this branch is the
        // one taken for negative hues.
        // dest = { m, c + m, x + m }
        store(m, add_ss(c, m), add_ss(x, m));
    } else if h >= 180.0 && h < 240.0 {
        // dest = { m, x + m, c + m }
        store(m, add_ss(x, m), add_ss(c, m));
    } else if h >= 240.0 && h < 300.0 {
        // dest = { x + m, m, c + m }
        store(add_ss(x, m), m, add_ss(c, m));
    } else if h >= 300.0 && h < 360.0 {
        // dest = { c + m, m, x + m }
        store(add_ss(c, m), m, add_ss(x, m));
    } else {
        // Hues outside [0, 360) that reach here (>= 360, or NaN) yield grey.
        store(m, m, m);
    }
}
