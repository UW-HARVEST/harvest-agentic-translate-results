#[unsafe(no_mangle)]
pub extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    unsafe {
        let src = std::slice::from_raw_parts(src, 3);
        let dest = std::slice::from_raw_parts_mut(dest, 3);

        let h = src[0];
        let s = src[1];
        let l = src[2];

        if s == 0.0 {
            dest[0] = l;
            dest[1] = l;
            dest[2] = l;
            return;
        }

        let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
        let m = l - 0.5f32 * c;
        let x = c * (1.0f32 - ((h / 60.0f32).rem_euclid(2.0f32) - 1.0f32).abs());

        if h >= 0.0 && h < 60.0 {
            dest[0] = c + m;
            dest[1] = x + m;
            dest[2] = m;
        } else if h >= 60.0 && h < 120.0 {
            dest[0] = x + m;
            dest[1] = c + m;
            dest[2] = m;
        } else if h < 120.0 && h < 180.0 {
            dest[0] = m;
            dest[1] = c + m;
            dest[2] = x + m;
        } else if h >= 180.0 && h < 240.0 {
            dest[0] = m;
            dest[1] = x + m;
            dest[2] = c + m;
        } else if h >= 240.0 && h < 300.0 {
            dest[0] = x + m;
            dest[1] = m;
            dest[2] = c + m;
        } else if h >= 300.0 && h < 360.0 {
            dest[0] = c + m;
            dest[1] = m;
            dest[2] = x + m;
        } else {
            dest[0] = m;
            dest[1] = m;
            dest[2] = m;
        }
    }
}
