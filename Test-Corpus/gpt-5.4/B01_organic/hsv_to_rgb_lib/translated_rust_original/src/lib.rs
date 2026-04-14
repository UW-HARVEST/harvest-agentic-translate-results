#[unsafe(no_mangle)]
pub extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    if dest.is_null() || src.is_null() {
        return;
    }

    let src = unsafe { std::slice::from_raw_parts(src, 3) };
    let dest = unsafe { std::slice::from_raw_parts_mut(dest, 3) };

    let h = src[0];
    let s = src[1];
    let v = src[2];

    if s == 0.0 {
        dest[0] = v;
        dest[1] = v;
        dest[2] = v;
        return;
    }

    let h = h / 60.0_f32;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    dest[0] = r;
    dest[1] = g;
    dest[2] = b;
}
