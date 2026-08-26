#[link(name = "m")]
unsafe extern "C" {
    fn fabsf(x: f32) -> f32;
    fn fmodf(x: f32, y: f32) -> f32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut f32, src: *const f32) {
    let h = unsafe { *src.add(0) };
    let s = unsafe { *src.add(1) };
    let l = unsafe { *src.add(2) };
    let c: f32;
    let m: f32;
    let x: f32;

    if s == 0.0f32 {
        unsafe {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    c = (1.0f32 - unsafe { fabsf(2.0f32 * l - 1.0f32) }) * s;
    m = 1.0f32 * (l - 0.5f32 * c);
    x = c * (1.0f32 - unsafe { fabsf(fmodf(h / 60.0f32, 2.0f32) - 1.0f32) });

    unsafe {
        if h >= 0.0f32 && h < 60.0f32 {
            *dest.add(0) = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        } else if h >= 60.0f32 && h < 120.0f32 {
            *dest.add(0) = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        } else if h < 120.0f32 && h < 180.0f32 {
            *dest.add(0) = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        } else if h >= 180.0f32 && h < 240.0f32 {
            *dest.add(0) = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        } else if h >= 240.0f32 && h < 300.0f32 {
            *dest.add(0) = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        } else if h >= 300.0f32 && h < 360.0f32 {
            *dest.add(0) = c + m;
            *dest.add(1) = m;
            *dest.add(2) = x + m;
        } else {
            *dest.add(0) = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}
