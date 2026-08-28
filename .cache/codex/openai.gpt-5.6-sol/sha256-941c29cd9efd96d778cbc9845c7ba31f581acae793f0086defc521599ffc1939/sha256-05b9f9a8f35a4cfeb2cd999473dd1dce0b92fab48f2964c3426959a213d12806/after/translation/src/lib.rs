use std::ffi::{c_int, c_void};

#[link(name = "m")]
unsafe extern "C" {
    fn sqrtf(value: f32) -> f32;
}

unsafe extern "C" {
    fn memset(destination: *mut c_void, value: c_int, count: usize) -> *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let mut sum = 0.0_f32;
    let mut i = 0;

    while i < size {
        let value = unsafe { src.offset(i as isize).read() };
        sum += value * value;
        i += 1;
    }

    if sum > 0.0 {
        sum = 1.0 / unsafe { sqrtf(sum) };
        i = 0;
        while i < size {
            let value = unsafe { src.offset(i as isize).read() };
            unsafe { dest.offset(i as isize).write(value * sum) };
            i += 1;
        }
    } else if dest.cast_const() != src {
        let byte_count = (size as usize).wrapping_mul(size_of::<f32>());
        unsafe {
            memset(dest.cast(), 0, byte_count);
        }
    }
}
