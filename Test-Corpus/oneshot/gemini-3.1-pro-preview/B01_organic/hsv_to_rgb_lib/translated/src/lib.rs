use std::os::raw::c_float;

#[unsafe(no_mangle)]
pub extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    unsafe {
        let h = *src.add(0);
        let s = *src.add(1);
        let v = *src.add(2);

        if s == 0.0 {
            *dest.add(0) = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
            return;
        }

        let h_div = h / 60.0;
        let i = h_div.floor() as i32;
        let f = h_div - (i as f32);
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

        *dest.add(0) = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}
