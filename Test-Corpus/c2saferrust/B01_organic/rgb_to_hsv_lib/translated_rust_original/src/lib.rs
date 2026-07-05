
#[no_mangle]
pub fn rgb_to_hsv(dest: &mut [f32; 3], src: &[f32; 3]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];

    let mut h = 0.0f32;
    let s;
    let v;

    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let delta = max - min;

    v = max;

    if delta == 0.0 || max == 0.0 {
        dest[0] = h;
        dest[1] = 0.0;
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

