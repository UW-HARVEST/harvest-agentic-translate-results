//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   - `rgb_to_hsv`
//!
//! Semantics are reproduced literally, including the exact ternary-based
//! min/max expansions from the C source (which differ from `f32::min` /
//! `f32::max` in their NaN handling) and the original order of checks.

use std::ffi::c_float;

/// Convert an RGB triple to HSV.
///
/// Direct translation of `void rgb_to_hsv(float *dest, const float *src)`.
///
/// `src` must point to at least 3 readable `float`s and `dest` to at least 3
/// writable `float`s; the C original performs no validation and neither does
/// this translation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    let r: f32 = *src.add(0);
    let g: f32 = *src.add(1);
    let b: f32 = *src.add(2);

    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;

    let mut min: f32 = r;
    let mut max: f32 = r;
    let delta: f32;

    // min = (((min) < (g)) ? (min) : (g));
    min = if min < g { min } else { g };
    // min = (((min) < (b)) ? (min) : (b));
    min = if min < b { min } else { b };
    // max = (((max) > (g)) ? (max) : (g));
    max = if max > g { max } else { g };
    // max = (((max) > (b)) ? (max) : (b));
    max = if max > b { max } else { b };

    delta = max - min;
    v = max;

    if delta == 0.0 || max == 0.0 {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
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

    *dest.add(0) = h;
    *dest.add(1) = s;
    *dest.add(2) = v;
}
