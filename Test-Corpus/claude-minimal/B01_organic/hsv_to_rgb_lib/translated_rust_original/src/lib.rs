use std::os::raw::c_float;

/// Converts an HSV color to RGB.
///
/// # Safety
///
/// `dest` must be a valid pointer to at least 3 writable `f32` values.
/// `src` must be a valid pointer to at least 3 readable `f32` values.
#[no_mangle]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    let mut h: f32 = *src.offset(0);
    let s: f32 = *src.offset(1);
    let v: f32 = *src.offset(2);

    if s == 0.0 {
        *dest.offset(0) = v;
        *dest.offset(1) = v;
        *dest.offset(2) = v;
        return;
    }

    h /= 60.0_f32;
    let i: i32 = h.floor() as i32;
    let f: f32 = h - (i as f32);
    let p: f32 = v * (1.0 - s);
    let q: f32 = v * (1.0 - s * f);
    let t: f32 = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    *dest.offset(0) = r;
    *dest.offset(1) = g;
    *dest.offset(2) = b;
}
