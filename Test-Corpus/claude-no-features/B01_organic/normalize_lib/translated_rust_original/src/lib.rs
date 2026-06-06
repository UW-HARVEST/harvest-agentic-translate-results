use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let mut sum: f32 = 0.0f32;
    let n = size as isize;

    let mut i: isize = 0;
    while i < n {
        let v = unsafe { *src.offset(i) };
        sum += v * v;
        i += 1;
    }

    if sum > 0.0f32 {
        sum = 1.0f32 / sum.sqrt();
        let mut i: isize = 0;
        while i < n {
            let v = unsafe { *src.offset(i) };
            unsafe { *dest.offset(i) = v * sum; }
            i += 1;
        }
    } else if dest as *const f32 != src {
        // memset(dest, 0, size * sizeof(float)) -> all bytes zero, equivalent to f32 0.0
        let byte_count = (size as usize).wrapping_mul(std::mem::size_of::<f32>());
        unsafe {
            std::ptr::write_bytes(dest as *mut u8, 0u8, byte_count);
        }
    }
}
