use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    if size <= 0 {
        return;
    }
    let size_usize = size as usize;
    let mut sum = 0.0f32;
    for i in 0..size_usize {
        let val = unsafe { *src.add(i) };
        sum += val * val;
    }
    if sum > 0.0f32 {
        let inv_sqrt = 1.0f32 / sum.sqrt();
        for i in 0..size_usize {
            unsafe {
                *dest.add(i) = *src.add(i) * inv_sqrt;
            }
        }
    } else if dest as *const f32 != src {
        unsafe {
            std::ptr::write_bytes(dest, 0, size_usize);
        }
    }
}
