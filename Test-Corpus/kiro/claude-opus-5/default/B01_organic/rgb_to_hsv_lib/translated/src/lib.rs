//! Rust translation of `c_src/src/lib.c`.
//!
//! The C source exposes a single function, `rgb_to_hsv`, with no namespace
//! renaming macros in `include/lib.h`, so the final linker symbol is
//! `rgb_to_hsv`.

use std::ffi::c_float;

/// Convert an RGB triple to HSV.
///
/// Direct translation of:
///
/// ```c
/// void rgb_to_hsv(float *dest, const float *src);
/// ```
///
/// `src` must point to at least 3 readable `float`s and `dest` to at least 3
/// writable `float`s. The comparisons below are written as explicit ternaries
/// rather than `f32::min`/`f32::max` so that NaN inputs propagate exactly the
/// way the C code does (a false comparison always selects the second operand).
///
/// # Safety
///
/// The caller must uphold the pointer requirements described above; this
/// mirrors the (unchecked) contract of the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    // Read the three input channels.
    let (r, g, b) = unsafe { (*src.add(0), *src.add(1), *src.add(2)) };

    let mut h: c_float = 0.0;
    let mut s: c_float = 0.0;
    let v: c_float;

    let mut min: c_float = r;
    let mut max: c_float = r;

    // min = (min < g) ? min : g;  min = (min < b) ? min : b;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };

    // max = (max > g) ? max : g;  max = (max > b) ? max : b;
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };

    let delta: c_float = max - min;
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
