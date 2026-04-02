unsafe extern "C" {
    fn fabsf(x: f32) -> f32;
    fn fmodf(x: f32, y: f32) -> f32;
}

#[unsafe(no_mangle)]
pub extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    unsafe {
        let h = *src;
        let s = *src.add(1);
        let l = *src.add(2);

        if s == 0.0f32 {
            *dest = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
            return;
        }

        let c = (1.0f32 - fabsf(2.0f32 * l - 1.0f32)) * s;
        let m = 1.0f32 * (l - 0.5f32 * c);
        let x = c * (1.0f32 - fabsf(fmodf(h / 60.0f32, 2.0f32) - 1.0f32));

        if h >= 0.0f32 && h < 60.0f32 {
            *dest = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        } else if h >= 60.0f32 && h < 120.0f32 {
            *dest = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        } else if h < 120.0f32 && h < 180.0f32 {
            // NOTE: reproduces C bug — first condition should be h >= 120.0f
            *dest = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        } else if h >= 180.0f32 && h < 240.0f32 {
            *dest = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        } else if h >= 240.0f32 && h < 300.0f32 {
            *dest = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        } else if h >= 300.0f32 && h < 360.0f32 {
            *dest = c + m;
            *dest.add(1) = m;
            *dest.add(2) = x + m;
        } else {
            *dest = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}
