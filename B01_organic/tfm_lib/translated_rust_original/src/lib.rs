use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut f32, src: *const f32, count: c_int) {
    unsafe {
        let mut src = src;
        let mut dest = dest;
        for _ in 0..count {
            if *src.add(0) < *src.add(1) {
                let dx2 = *src.add(0);
                let dy2 = *src.add(1);
                let dxy = *src.add(2);
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                    + (4.0f32 * dxy * dxy);
                let max = if 0.0f32 > sqd { 0.0f32 } else { sqd };
                let lambda = 0.5f32 * (dy2 + dx2 + max.sqrt());
                *dest.add(0) = dx2 - lambda;
                *dest.add(1) = dxy;
            } else {
                let dy2 = *src.add(0);
                let dx2 = *src.add(1);
                let dxy = *src.add(2);
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2)
                    + (4.0f32 * dxy * dxy);
                let max = if 0.0f32 > sqd { 0.0f32 } else { sqd };
                let lambda = 0.5f32 * (dy2 + dx2 + max.sqrt());
                *dest.add(0) = dxy;
                *dest.add(1) = dx2 - lambda;
            }
            src = src.add(3);
            dest = dest.add(2);
        }
    }
}
