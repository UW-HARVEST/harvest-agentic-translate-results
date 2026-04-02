use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let n = size as usize;
    let mut sum: f32 = 0.0;
    for i in 0..n {
        let v = unsafe { *src.add(i) };
        sum += v * v;
    }
    if sum > 0.0 {
        sum = 1.0 / sum.sqrt();
        for i in 0..n {
            unsafe {
                *dest.add(i) = *src.add(i) * sum;
            }
        }
    } else if dest as *const f32 != src {
        unsafe {
            core::ptr::write_bytes(dest, 0, n);
        }
    }
}
