#[unsafe(no_mangle)]
pub extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    unsafe {
        let h = *src;
        let s = *src.add(1);
        let l = *src.add(2);

        if s == 0.0 {
            *dest = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
            return;
        }

        let c = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
        let m = 1.0f32 * (l - 0.5f32 * c);
        let x = c * (1.0f32 - ((h / 60.0f32) % 2.0 - 1.0f32).abs());

        if h >= 0.0 && h < 60.0 {
            *dest = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        } else if h >= 60.0 && h < 120.0 {
            *dest = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        } else if h < 120.0 && h < 180.0 {
            // Bug preserved: should be h >= 120.0
            *dest = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        } else if h >= 180.0 && h < 240.0 {
            *dest = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        } else if h >= 240.0 && h < 300.0 {
            *dest = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        } else if h >= 300.0 && h < 360.0 {
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
