use std::slice;

#[unsafe(no_mangle)]
pub extern "C" fn rgb_to_hsv(dest: *mut f32, src: *const f32) {
    let src = unsafe { slice::from_raw_parts(src, 3) };
    let dest = unsafe { slice::from_raw_parts_mut(dest, 3) };

    let r = src[0];
    let g = src[1];
    let b = src[2];
    let mut h = 0.0f32;
    let mut s = 0.0f32;
    let mut v;
    let mut min = r;
    let mut max = r;

    min = min.min(g);
    min = min.min(b);
    max = max.max(g);
    max = max.max(b);

    let delta = max - min;
    v = max;

    if delta == 0.0 || max == 0.0 {
        dest[0] = h;
        dest[1] = s;
        dest[2] = v;
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

    dest[0] = h;
    dest[1] = s;
    dest[2] = v;
}
