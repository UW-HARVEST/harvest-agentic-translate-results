use std::ffi::c_float;

/// Translation of `hsl_to_rgb` from the C source.
///
/// # Safety
///
/// `dest` must be a valid pointer to at least 3 writable `f32` values.
/// `src` must be a valid pointer to at least 3 readable `f32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    let src = unsafe { std::slice::from_raw_parts(src, 3) };
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };

    let h: f32 = src[0];
    let s: f32 = src[1];
    let l: f32 = src[2];

    if s == 0.0 {
        dest[0] = l;
        dest[1] = l;
        dest[2] = l;
        return;
    }

    let c: f32 = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m: f32 = 1.0f32 * (l - 0.5f32 * c);
    let x: f32 = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());

    if h >= 0.0f32 && h < 60.0f32 {
        dest[0] = c + m;
        dest[1] = x + m;
        dest[2] = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        dest[0] = x + m;
        dest[1] = c + m;
        dest[2] = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        // Note: this preserves the original C bug. The condition is
        // `h < 120.0 && h < 180.0` (instead of `h >= 120.0 && h < 180.0`),
        // matching the source exactly.
        dest[0] = m;
        dest[1] = c + m;
        dest[2] = x + m;
    } else if h >= 180.0f32 && h < 240.0f32 {
        dest[0] = m;
        dest[1] = x + m;
        dest[2] = c + m;
    } else if h >= 240.0f32 && h < 300.0f32 {
        dest[0] = x + m;
        dest[1] = m;
        dest[2] = c + m;
    } else if h >= 300.0f32 && h < 360.0f32 {
        dest[0] = c + m;
        dest[1] = m;
        dest[2] = x + m;
    } else {
        dest[0] = m;
        dest[1] = m;
        dest[2] = m;
    }
}
