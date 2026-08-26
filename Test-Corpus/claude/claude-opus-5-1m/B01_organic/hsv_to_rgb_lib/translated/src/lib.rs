//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! Source: `c_src/src/lib.c`, header: `c_src/include/lib.h`
//!
//! Behaviour is reproduced exactly, including the platform-specific result of
//! the C `(int)` cast of a `float` (see [`cvt_f32_to_i32`]).

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;

/// Reproduce the x86-64 `cvttss2si` semantics that GCC/Clang emit for the C
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
fn cvt_f32_to_i32(x: f32) -> c_int {
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
/// # Safety
///
/// `src` must be valid for reads of 3 `f32`s and `dest` valid for writes of
/// 3 `f32`s, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;

    let mut h: f32 = unsafe { *src.add(0) };
    let s: f32 = unsafe { *src.add(1) };
    let v: f32 = unsafe { *src.add(2) };
    let i: c_int;

    if s == 0.0f32 {
        unsafe {
            *dest.add(0) = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
        }
        return;
    }

    h /= 60.0f32;
    i = cvt_f32_to_i32(libm_floorf(h));
    f = h - (i as f32);
    p = v * (1.0f32 - s);
    q = v * (1.0f32 - s * f);
    t = v * (1.0f32 - s * (1.0f32 - f));

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
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}

/// `floorf` from `<math.h>`.
///
/// `f32::floor` is documented to match the IEEE-754 `roundToIntegralTowardNegative`
/// operation, which is exactly what C's `floorf` computes (including the
/// sign-preserving behaviour for zeros and the identity mapping for NaN/inf),
/// so it is a faithful stand-in and avoids a libm dependency.
#[inline]
fn libm_floorf(x: f32) -> f32 {
    x.floor()
}
