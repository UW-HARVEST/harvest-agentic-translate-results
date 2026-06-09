pub fn rgb_to_hsv(dest: &mut [f32], src: &[f32]) {
    let r = src[0];
    let g = src[1];
    let b = src[2];
    let mut h: f32 = 0.0;
    let mut s: f32 = 0.0;
    let v: f32;
    let mut min = r;
    let mut max = r;
    let delta;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    delta = max - min;
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
