use std::os::raw::c_float;

/// Convert HSL color to RGB.
///
/// # Safety
///
/// `dest` must point to a writable buffer of at least 3 `f32` values.
/// `src` must point to a readable buffer of at least 3 `f32` values.
#[no_mangle]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    let h = *src.offset(0);
    let s = *src.offset(1);
    let l = *src.offset(2);

    if s == 0.0 {
        *dest.offset(0) = l;
        *dest.offset(1) = l;
        *dest.offset(2) = l;
        return;
    }

    let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m = 1.0f32 * (l - 0.5f32 * c);
    let x = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());

    if h >= 0.0 && h < 60.0 {
        *dest.offset(0) = c + m;
        *dest.offset(1) = x + m;
        *dest.offset(2) = m;
    } else if h >= 60.0 && h < 120.0 {
        *dest.offset(0) = x + m;
        *dest.offset(1) = c + m;
        *dest.offset(2) = m;
    } else if h < 120.0 && h < 180.0 {
        *dest.offset(0) = m;
        *dest.offset(1) = c + m;
        *dest.offset(2) = x + m;
    } else if h >= 180.0 && h < 240.0 {
        *dest.offset(0) = m;
        *dest.offset(1) = x + m;
        *dest.offset(2) = c + m;
    } else if h >= 240.0 && h < 300.0 {
        *dest.offset(0) = x + m;
        *dest.offset(1) = m;
        *dest.offset(2) = c + m;
    } else if h >= 300.0 && h < 360.0 {
        *dest.offset(0) = c + m;
        *dest.offset(1) = m;
        *dest.offset(2) = x + m;
    } else {
        *dest.offset(0) = m;
        *dest.offset(1) = m;
        *dest.offset(2) = m;
    }
}
