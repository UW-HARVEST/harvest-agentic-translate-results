use std::ffi::{c_float, c_int};
use std::{mem, ptr};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut c_float, src: *const c_float, size: c_int) {
    let mut sum = 0.0_f32;
    let mut i = 0;

    while i < size {
        // SAFETY: The C ABI requires src to reference at least size floats.
        let value = unsafe { ptr::read(src.offset(i as isize)) };
        sum += value * value;
        i += 1;
    }

    if sum > 0.0 {
        sum = 1.0 / sum.sqrt();
        i = 0;
        while i < size {
            // Read immediately before each write to preserve C's behavior for
            // partially overlapping source and destination buffers.
            let value = unsafe { ptr::read(src.offset(i as isize)) };
            // SAFETY: The C ABI requires dest to reference at least size floats.
            unsafe { ptr::write(dest.offset(i as isize), value * sum) };
            i += 1;
        }
    } else if !ptr::eq(dest.cast_const(), src) {
        let byte_count = (size as usize).wrapping_mul(mem::size_of::<c_float>());
        // SAFETY: This has the same pointer and converted byte count as C memset.
        unsafe { ptr::write_bytes(dest.cast::<u8>(), 0, byte_count) };
    }
}
