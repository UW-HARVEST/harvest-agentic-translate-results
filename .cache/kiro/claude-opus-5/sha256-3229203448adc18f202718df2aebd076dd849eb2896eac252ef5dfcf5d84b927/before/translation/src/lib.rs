//! Rust translation of `c_src/src/lib.c`.
//!
//! Behaviour is intended to be bit-for-bit identical to the original C, which
//! is compiled for x86-64 (SSE single-precision arithmetic, `cvttss2si` for the
//! float -> int conversion).

use std::ffi::c_float;

/// Reproduces the C expression `(int)floorf(h)`.
///
/// A C cast from a floating point value that is out of range for `int` is
/// undefined behaviour; on x86-64 the generated `cvttss2si` instruction yields
/// `INT_MIN` (the "integer indefinite" value) for out-of-range inputs and for
/// NaN. Rust's `as` cast saturates instead, so the conversion is emulated here
/// to keep the observable behaviour the same as the C build.
#[inline]
fn c_floor_to_int(h: c_float) -> i32 {
    let floored = h.floor();
    // Both comparisons are false for NaN, which correctly falls through to
    // the indefinite value.
    if floored >= -2_147_483_648.0f32 && floored < 2_147_483_648.0f32 {
        floored as i32
    } else {
        i32::MIN
    }
}

/// Converts an HSV triple to RGB.
///
/// `src` points to three floats `[h, s, v]`; `dest` receives three floats
/// `[r, g, b]`.
///
/// # Safety
///
/// `dest` must be valid for writes of three `f32` values and `src` valid for
/// reads of three `f32` values, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    let src = unsafe { std::slice::from_raw_parts(src, 3) };
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };

    let mut h = src[0];
    let s = src[1];
    let v = src[2];

    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }

    h /= 60.0f32;
    let i = c_floor_to_int(h);
    let f = h - i as c_float;
    let p = v * (1.0f32 - s);
    let q = v * (1.0f32 - s * f);
    let t = v * (1.0f32 - s * (1.0f32 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}
