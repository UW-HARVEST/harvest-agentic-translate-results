use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn tfm(mut dest: *mut f32, mut src: *const f32, count: c_int) {
    for _ in 0..count {
        unsafe {
            let s0 = *src;
            let s1 = *src.add(1);
            let s2 = *src.add(2);
            if s0 < s1 {
                let dx2 = s0;
                let dy2 = s1;
                let dxy = s2;
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                    + (4.0f32 * dxy * dxy);
                let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0f32).sqrt());
                *dest = dx2 - lambda;
                *dest.add(1) = dxy;
            } else {
                let dy2 = s0;
                let dx2 = s1;
                let dxy = s2;
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                    + (4.0f32 * dxy * dxy);
                let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0f32).sqrt());
                *dest = dxy;
                *dest.add(1) = dx2 - lambda;
            }
            src = src.add(3);
            dest = dest.add(2);
        }
    }
}
