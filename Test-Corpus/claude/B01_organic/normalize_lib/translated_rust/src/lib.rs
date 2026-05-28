use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let n = size as isize;
    let mut sum: f32 = 0.0f32;
    let mut i: isize = 0;
    while i < n {
        let v = unsafe { *src.offset(i) };
        sum += v * v;
        i += 1;
    }
    if sum > 0.0f32 {
        let inv = 1.0f32 / sum.sqrt();
        let mut j: isize = 0;
        while j < n {
            let v = unsafe { *src.offset(j) };
            unsafe { *dest.offset(j) = v * inv };
            j += 1;
        }
    } else if dest as *const f32 != src {
        // memset dest to 0 over `size * sizeof(float)` bytes
        if n > 0 {
            unsafe {
                std::ptr::write_bytes(dest, 0u8, n as usize);
            }
        }
    }
}
