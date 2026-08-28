//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (mirrors `c_src/include/lib.h`):
//!   void rgb_to_hsv(float *dest, const float *src);
//!
//! The translation is intentionally literal: operation order, comparison
//! direction (which matters for NaN inputs), the `delta == 0 || max == 0`
//! early-out, and the order of the stores into `dest` all match the C exactly.

#![allow(clippy::missing_safety_doc)]

use std::ffi::c_float;

/// Literal translation of the C `MIN(a, b)` expansion
/// `(((a) < (b)) ? (a) : (b))`.
///
/// This is *not* `f32::min`: for NaN operands the C ternary yields `b`
/// whenever the `<` comparison is false, which differs from `f32::min`'s
/// NaN-suppressing behaviour. Reproducing the C exactly requires the
/// raw comparison.
#[inline]
fn c_min(a: c_float, b: c_float) -> c_float {
    if a < b {
        a
    } else {
        b
    }
}

/// Literal translation of the C `MAX(a, b)` expansion
/// `(((a) > (b)) ? (a) : (b))`. See [`c_min`] for why `f32::max` is avoided.
#[inline]
fn c_max(a: c_float, b: c_float) -> c_float {
    if a > b {
        a
    } else {
        b
    }
}

/// Convert an RGB triple to HSV.
///
/// `src` must point to at least 3 readable `float`s, `dest` to at least 3
/// writable `float`s — the same contract the C function imposes.
///
/// Hue is returned in degrees `[0, 360)`, saturation and value in `[0, 1]`
/// for in-range inputs. Out-of-range inputs are not validated, matching C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    // float r = src[0]; float g = src[1]; float b = src[2];
    // All three loads happen up front, exactly as in the C, so an aliasing
    // `dest == src` call behaves identically.
    let r: c_float = unsafe { *src.add(0) };
    let g: c_float = unsafe { *src.add(1) };
    let b: c_float = unsafe { *src.add(2) };

    let mut h: c_float = 0.0;
    let mut s: c_float = 0.0;
    let v: c_float;

    // float min = r; float max = r;
    let mut min: c_float = r;
    let mut max: c_float = r;
    let delta: c_float;

    min = c_min(min, g);
    min = c_min(min, b);
    max = c_max(max, g);
    max = c_max(max, b);

    delta = max - min;
    v = max;

    if delta == 0.0 || max == 0.0 {
        unsafe {
            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
        }
        return;
    }

    s = delta / max;

    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2.0 + (b - r) / delta;
    } else {
        h = 4.0 + (r - g) / delta;
    }

    h *= 60.0;

    if h < 0.0 {
        h += 360.0;
    }

    unsafe {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}
