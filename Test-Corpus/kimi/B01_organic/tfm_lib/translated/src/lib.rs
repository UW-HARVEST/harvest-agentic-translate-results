use std::os::raw::{c_float, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tfm(dest: *mut c_float, src: *const c_float, count: c_int) {
    let mut src_ptr = src;
    let mut dest_ptr = dest;
    
    for _ in 0..count {
        unsafe {
            let src0 = *src_ptr;
            let src1 = *src_ptr.add(1);
            let src2 = *src_ptr.add(2);
            
            if src0 < src1 {
                let dx2 = src0;
                let dy2 = src1;
                let dxy = src2;
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) +
                          (4.0f32 * dxy * dxy);
                let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0f32).sqrt());
                *dest_ptr = dx2 - lambda;
                *dest_ptr.add(1) = dxy;
            } else {
                let dy2 = src0;
                let dx2 = src1;
                let dxy = src2;
                let sqd = (dy2 * dy2) - (2.0f32 * dx2 * dy2) + (dx2 * dx2) +
                          (4.0f32 * dxy * dxy);
                let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0f32).sqrt());
                *dest_ptr = dxy;
                *dest_ptr.add(1) = dx2 - lambda;
            }
            
            src_ptr = src_ptr.add(3);
            dest_ptr = dest_ptr.add(2);
        }
    }
}
