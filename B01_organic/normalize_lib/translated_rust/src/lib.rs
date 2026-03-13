use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let size = size as usize;
    unsafe {
        let src_slice = std::slice::from_raw_parts(src, size);
        let dest_slice = std::slice::from_raw_parts_mut(dest, size);

        let mut sum: f32 = 0.0;
        for i in 0..size {
            sum += src_slice[i] * src_slice[i];
        }

        if sum > 0.0 {
            sum = 1.0 / sum.sqrt();
            for i in 0..size {
                dest_slice[i] = src_slice[i] * sum;
            }
        } else if dest as *const f32 != src {
            for v in dest_slice.iter_mut() {
                *v = 0.0;
            }
        }
    }
}
