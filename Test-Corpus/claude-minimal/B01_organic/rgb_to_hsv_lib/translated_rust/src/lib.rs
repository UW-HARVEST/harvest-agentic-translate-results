use std::os::raw::c_float;

/// Converts an RGB color to HSV.
///
/// # Safety
///
/// `dest` must be a valid pointer to an array of at least 3 `f32` values.
/// `src` must be a valid pointer to an array of at least 3 `f32` values.
#[no_mangle]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    let r = *src.offset(0);
    let g = *src.offset(1);
    let b = *src.offset(2);
    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;
    let mut min = r;
    let mut max = r;
    let delta;

    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    delta = max - min;
    v = max;

    if delta == 0.0 || max == 0.0 {
        *dest.offset(0) = h;
        *dest.offset(1) = s;
        *dest.offset(2) = v;
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
    *dest.offset(0) = h;
    *dest.offset(1) = s;
    *dest.offset(2) = v;
}
