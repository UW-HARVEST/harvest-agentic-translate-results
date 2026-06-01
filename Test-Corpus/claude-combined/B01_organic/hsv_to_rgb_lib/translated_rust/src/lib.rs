use std::ffi::c_float;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;
    let mut h: f32 = unsafe { *src.add(0) };
    let s: f32 = unsafe { *src.add(1) };
    let v: f32 = unsafe { *src.add(2) };
    let i: i32;

    if s == 0.0 {
        unsafe {
            *dest.add(0) = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
        }
        return;
    }

    h /= 60.0f32;
    i = h.floor() as i32;
    f = h - (i as f32);
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
        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}
