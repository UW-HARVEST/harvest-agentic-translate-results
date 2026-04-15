use std::os::raw::c_float;

#[unsafe(no_mangle)]
pub extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    unsafe {
        let h = *src.add(0);
        let s = *src.add(1);
        let l = *src.add(2);

        if s == 0.0 {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
            return;
        }

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let m = 1.0 * (l - 0.5 * c);
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());

        if h >= 0.0 && h < 60.0 {
            *dest.add(0) = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        } else if h >= 60.0 && h < 120.0 {
            *dest.add(0) = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        } else if h < 120.0 && h < 180.0 {
            *dest.add(0) = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        } else if h >= 180.0 && h < 240.0 {
            *dest.add(0) = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        } else if h >= 240.0 && h < 300.0 {
            *dest.add(0) = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        } else if h >= 300.0 && h < 360.0 {
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
