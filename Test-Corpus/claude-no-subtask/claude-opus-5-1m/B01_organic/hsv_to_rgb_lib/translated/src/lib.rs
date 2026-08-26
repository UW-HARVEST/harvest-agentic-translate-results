use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    let mut h: f32 = unsafe { *src.offset(0) };
    let s: f32 = unsafe { *src.offset(1) };
    let v: f32 = unsafe { *src.offset(2) };

    if s == 0.0 {
        unsafe {
            *dest.offset(0) = v;
            *dest.offset(1) = v;
            *dest.offset(2) = v;
        }
        return;
    }

    h /= 60.0f32;
    let i: c_int = h.floor() as c_int;
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

    unsafe {
        *dest.offset(0) = r;
        *dest.offset(1) = g;
        *dest.offset(2) = b;
    }
}
