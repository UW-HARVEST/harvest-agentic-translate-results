#[unsafe(no_mangle)]
pub extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    unsafe {
        let h_orig = *src;
        let s = *src.add(1);
        let v = *src.add(2);

        if s == 0.0 {
            *dest = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
            return;
        }

        let h = h_orig / 60.0f32;
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

        *dest = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}
