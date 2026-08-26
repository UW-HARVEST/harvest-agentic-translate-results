use std::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let n = size as isize;
    let mut sum: f32 = 0.0;
    let mut i: isize = 0;
    while i < n {
        let v = *src.offset(i);
        sum += v * v;
        i += 1;
    }
    if sum > 0.0 {
        sum = 1.0 / sum.sqrt();
        i = 0;
        while i < n {
            *dest.offset(i) = *src.offset(i) * sum;
            i += 1;
        }
    } else if dest as *const f32 != src {
        if n > 0 {
            std::ptr::write_bytes(dest, 0u8, n as usize);
        }
    }
}
