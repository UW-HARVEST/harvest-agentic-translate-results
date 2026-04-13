use std::os::raw::c_float;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rgb_to_hsv(dest: *mut c_float, src: *const c_float) {
    let r = unsafe { *src.add(0) };
    let g = unsafe { *src.add(1) };
    let b = unsafe { *src.add(2) };
    let mut h = 0.0_f32;
    let mut s = 0.0_f32;
    let v: f32;
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let delta = max - min;
    v = max;
    if delta == 0.0 || max == 0.0 {
        unsafe {
            *dest.add(0) = h;
            *dest.add(1) = s;
            *dest.add(2) = v;
        }
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
    unsafe {
        *dest.add(0) = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}
