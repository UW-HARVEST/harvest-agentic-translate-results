#[unsafe(no_mangle)]
pub extern "C" fn rgb_to_hsv(dest: *mut f32, src: *const f32) {
    unsafe {
        let r = *src;
        let g = *src.add(1);
        let b = *src.add(2);
        let mut h: f32 = 0.0;
        let s;
        let v;
        let mut min = r;
        let mut max = r;
        if min > g { min = g; }
        if min > b { min = b; }
        if max < g { max = g; }
        if max < b { max = b; }
        let delta = max - min;
        v = max;
        if delta == 0.0 || max == 0.0 {
            *dest = h;
            *dest.add(1) = 0.0;
            *dest.add(2) = v;
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
        *dest = h;
        *dest.add(1) = s;
        *dest.add(2) = v;
    }
}
