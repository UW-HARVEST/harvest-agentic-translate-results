use std::ffi::{c_float, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(mut dest: *mut c_float, mut src: *const c_float, count: c_int) {
    let mut i: c_int = 0;
    while i < count {
        let src0 = unsafe { *src.add(0) };
        let src1 = unsafe { *src.add(1) };
        let src2 = unsafe { *src.add(2) };

        if src0 < src1 {
            let dx2 = src0;
            let dy2 = src1;
            let dxy = src2;
            let sqd = (dy2 * dy2) - (2.0_f32 * dx2 * dy2) + (dx2 * dx2) + (4.0_f32 * dxy * dxy);
            let lambda = 0.5_f32 * (dy2 + dx2 + (if 0.0_f32 > sqd { 0.0_f32 } else { sqd }).sqrt());
            unsafe {
                *dest.add(0) = dx2 - lambda;
                *dest.add(1) = dxy;
            }
        } else {
            let dy2 = src0;
            let dx2 = src1;
            let dxy = src2;
            let sqd = (dy2 * dy2) - (2.0_f32 * dx2 * dy2) + (dx2 * dx2) + (4.0_f32 * dxy * dxy);
            let lambda = 0.5_f32 * (dy2 + dx2 + (if 0.0_f32 > sqd { 0.0_f32 } else { sqd }).sqrt());
            unsafe {
                *dest.add(0) = dxy;
                *dest.add(1) = dx2 - lambda;
            }
        }

        src = unsafe { src.add(3) };
        dest = unsafe { dest.add(2) };
        i += 1;
    }
}
