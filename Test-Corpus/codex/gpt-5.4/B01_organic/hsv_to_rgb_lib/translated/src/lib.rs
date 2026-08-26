#[link(name = "m")]
unsafe extern "C" {
    fn floorf(x: f32) -> f32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    let r: f32;
    let g: f32;
    let b: f32;
    let f: f32;
    let p: f32;
    let q: f32;
    let t: f32;
    let mut h = unsafe { *src.add(0) };
    let s = unsafe { *src.add(1) };
    let v = unsafe { *src.add(2) };
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
    i = unsafe { floorf(h) as i32 };
    f = h - i as f32;
    p = v * (1.0f32 - s);
    q = v * (1.0f32 - s * f);
    t = v * (1.0f32 - s * (1.0f32 - f));

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
