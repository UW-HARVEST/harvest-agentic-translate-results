use std::ffi::{c_float, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut c_float, src: *const c_float, size: c_int) {
    let mut sum: c_float = 0.0;
    let mut i = 0;

    while i < size {
        let value = unsafe { *src.add(i as usize) };
        sum += value * value;
        i += 1;
    }

    if sum > 0.0 {
        sum = 1.0 / sum.sqrt();
        i = 0;
        while i < size {
            unsafe {
                *dest.add(i as usize) = *src.add(i as usize) * sum;
            }
            i += 1;
        }
    } else if !std::ptr::eq(dest as *const c_float, src) {
        unsafe {
            std::ptr::write_bytes(dest, 0, size as usize);
        }
    }
}
