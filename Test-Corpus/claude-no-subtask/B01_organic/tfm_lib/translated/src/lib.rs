use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    let mut src = src;
    let mut dest = dest;
    let mut i: c_int = 0;
    while i < count {
        let s0 = unsafe { *src };
        let s1 = unsafe { *src.add(1) };
        let s2 = unsafe { *src.add(2) };

        if s0 < s1 {
            let dx2 = s0;
            let dy2 = s1;
            let dxy = s2;
            let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                + (4.0f32 * dxy * dxy);
            let m = if 0.0f32 > sqd { 0.0f32 } else { sqd };
            let lambda = 0.5f32 * (dy2 + dx2 + m.sqrt());
            unsafe {
                *dest = dx2 - lambda;
                *dest.add(1) = dxy;
            }
        } else {
            let dy2 = s0;
            let dx2 = s1;
            let dxy = s2;
            let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                + (4.0f32 * dxy * dxy);
            let m = if 0.0f32 > sqd { 0.0f32 } else { sqd };
            let lambda = 0.5f32 * (dy2 + dx2 + m.sqrt());
            unsafe {
                *dest = dxy;
                *dest.add(1) = dx2 - lambda;
            }
        }

        src = unsafe { src.add(3) };
        dest = unsafe { dest.add(2) };
        i += 1;
    }
}
