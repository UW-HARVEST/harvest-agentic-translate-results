//! Rust translation of `c_src/src/lib.c`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `hsv_to_rgb`
//!
//! Behaviour is reproduced exactly, including the original code's quirks
//! (no range clamping of the hue, `s == 0` exact float comparison, and the
//! `default:` switch arm being reached for any sector index outside `0..=4`,
//! which includes negative hues).

use std::ffi::c_float;

/// Emulates the C cast `(int)x` for a `float` on x86-64 / AArch64 with the
/// standard SSE / NEON conversion instructions.
///
/// The C standard leaves out-of-range float-to-int conversions undefined; the
/// hardware used by the reference build produces the "integer indefinite"
/// value `INT_MIN` for NaN and for anything outside `[INT_MIN, INT_MAX]`.
/// Rust's `as` cast instead saturates, so the out-of-range cases are handled
/// explicitly to keep the observable results identical.
#[inline]
fn c_float_to_int(x: f32) -> i32 {
    // 2147483648.0 == 2^31 is exactly representable as f32; -2^31 likewise.
    if x >= -2147483648.0f32 && x < 2147483648.0f32 {
        // In range: truncation toward zero, same as the C cast.
        x as i32
    } else {
        // Out of range or NaN.
        i32::MIN
    }
}

/// Convert an HSV triple to RGB.
///
/// `src` must point to at least 3 readable `float`s (`h`, `s`, `v`) and `dest`
/// to at least 3 writable `float`s. Hue is expressed in degrees; saturation and
/// value are passed through untouched in the achromatic case.
///
/// # Safety
///
/// Both pointers must be valid, non-null and properly aligned for at least
/// three `float` elements, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    let src = unsafe { std::slice::from_raw_parts(src, 3) };
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };

    let mut h: f32 = src[0];
    let s: f32 = src[1];
    let v: f32 = src[2];

    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }

    h /= 60.0f32;
    let i: i32 = c_float_to_int(h.floor());
    let f: f32 = h - (i as f32);
    let p: f32 = v * (1.0f32 - s);
    let q: f32 = v * (1.0f32 - s * f);
    let t: f32 = v * (1.0f32 - s * (1.0f32 - f));

    let (r, g, b): (f32, f32, f32) = match i {
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
