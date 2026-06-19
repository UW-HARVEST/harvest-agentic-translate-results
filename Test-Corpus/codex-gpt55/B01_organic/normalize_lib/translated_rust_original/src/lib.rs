use std::ffi::{c_float, c_int};
use std::mem;

#[link(name = "m")]
unsafe extern "C" {
    fn sqrtf(x: c_float) -> c_float;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut c_float, src: *const c_float, size: c_int) {
    let mut sum: c_float = 0.0;
    let mut i: c_int = 0;

    while i < size {
        let value = unsafe { *src.offset(i as isize) };
        sum += value * value;
        i += 1;
    }

    if sum > 0.0 {
        sum = 1.0 / unsafe { sqrtf(sum) };
        i = 0;
        while i < size {
            unsafe {
                *dest.offset(i as isize) = *src.offset(i as isize) * sum;
            }
            i += 1;
        }
    } else if !std::ptr::eq(dest as *const c_float, src) {
        let bytes = (size as usize).wrapping_mul(mem::size_of::<c_float>());
        unsafe {
            (dest as *mut u8).write_bytes(0, bytes);
        }
    }
}
