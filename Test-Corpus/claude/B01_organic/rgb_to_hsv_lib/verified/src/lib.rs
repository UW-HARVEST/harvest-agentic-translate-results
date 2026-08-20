//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared object):
//!   * `rgb_to_hsv`
//!
//! The float comparisons below intentionally mirror the C ternary
//! expressions (`(a < b) ? a : b`) rather than using `f32::min` /
//! `f32::max`, because the standard library helpers have different NaN
//! propagation semantics than the raw C comparisons.

/// Translation of:
/// ```c
/// void rgb_to_hsv(float *dest, const float *src);
/// ```
///
/// # Safety
///
/// `dest` must be valid for writes of 3 `f32`s and `src` valid for reads of
/// 3 `f32`s, exactly as required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut f32, src: *const f32) {
    let r: f32 = *src.add(0);
    let g: f32 = *src.add(1);
    let b: f32 = *src.add(2);

    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;

    let mut min: f32 = r;
    let mut max: f32 = r;
    let delta: f32;

    // min = MIN(min, g); min = MIN(min, b);
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };

    // max = MAX(max, g); max = MAX(max, b);
    max = if max > g { max } else { g };
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
