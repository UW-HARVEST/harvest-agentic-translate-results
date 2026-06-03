use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;
    let mut h: f32 = unsafe { *src.offset(0) };
    let s: f32 = unsafe { *src.offset(1) };
    let v: f32 = unsafe { *src.offset(2) };
    let i: c_int;
    if s == 0.0 {
        unsafe {
            *dest.offset(0) = v;
            *dest.offset(1) = v;
            *dest.offset(2) = v;
        }
        return;
    }
    h /= 60.0f32;
    i = h.floor() as c_int;
    f = h - i as f32;
    p = v * (1.0 - s);
    q = v * (1.0 - s * f);
    t = v * (1.0 - s * (1.0 - f));
    match i {
        0 => {
            r = v;
            g = t;
            b = p;
        }
        1 => {
            r = q;
            g = v;
            b = p;
        }
        2 => {
            r = p;
            g = v;
            b = t;
        }
        3 => {
            r = p;
            g = q;
            b = v;
        }
        4 => {
            r = t;
            g = p;
            b = v;
        }
        _ => {
            r = v;
            g = p;
            b = q;
        }
    }
    unsafe {
        *dest.offset(0) = r;
        *dest.offset(1) = g;
        *dest.offset(2) = b;
    }
}
