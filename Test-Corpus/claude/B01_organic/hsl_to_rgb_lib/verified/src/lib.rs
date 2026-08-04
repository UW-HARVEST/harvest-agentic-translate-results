use std::ffi::c_float;

extern "C" {
    fn fabsf(x: c_float) -> c_float;
    fn fmodf(x: c_float, y: c_float) -> c_float;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    let h: c_float = *src.offset(0);
    let s: c_float = *src.offset(1);
    let l: c_float = *src.offset(2);
    let c: c_float;
    let m: c_float;
    let x: c_float;
    if s == 0.0 {
        *dest.offset(0) = l;
        *dest.offset(1) = l;
        *dest.offset(2) = l;
        return;
    }
    c = (1.0f32 - fabsf(2.0f32 * l - 1.0f32)) * s;
    m = 1.0f32 * (l - 0.5f32 * c);
    x = c * (1.0f32 - fabsf(fmodf(h / 60.0f32, 2.0f32) - 1.0f32));
    if h >= 0.0f32 && h < 60.0f32 {
        *dest.offset(0) = c + m;
        *dest.offset(1) = x + m;
        *dest.offset(2) = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        *dest.offset(0) = x + m;
        *dest.offset(1) = c + m;
        *dest.offset(2) = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        *dest.offset(0) = m;
        *dest.offset(1) = c + m;
        *dest.offset(2) = x + m;
    } else if h >= 180.0f32 && h < 240.0f32 {
        *dest.offset(0) = m;
        *dest.offset(1) = x + m;
        *dest.offset(2) = c + m;
    } else if h >= 240.0f32 && h < 300.0f32 {
        *dest.offset(0) = x + m;
        *dest.offset(1) = m;
        *dest.offset(2) = c + m;
    } else if h >= 300.0f32 && h < 360.0f32 {
        *dest.offset(0) = c + m;
        *dest.offset(1) = m;
        *dest.offset(2) = x + m;
    } else {
        *dest.offset(0) = m;
        *dest.offset(1) = m;
        *dest.offset(2) = m;
    }
}
