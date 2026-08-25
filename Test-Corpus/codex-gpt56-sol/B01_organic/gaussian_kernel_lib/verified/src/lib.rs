use std::ffi::{c_float, c_int};

#[link(name = "m")]
unsafe extern "C" {
    fn expf(value: c_float) -> c_float;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut c_float, size: c_int, radius: c_float) {
    let sigma = 1.6_f32;
    let tetha = 2.25_f32;
    let hsize = size / 2;
    let s2 = 1.0_f32 / unsafe { expf(sigma * sigma * tetha) };
    let rs = sigma / radius;
    let mut k = dest;
    let mut sum = 0.0_f32;
    let mut r = -hsize;

    while r <= hsize {
        let x = (r as c_float) * rs;
        let mut v = (1.0_f32 / unsafe { expf(x * x) }) - s2;
        v = if v > 0.0_f32 { v } else { 0.0_f32 };
        unsafe {
            *k = v;
            k = k.add(1);
        }
        sum += v;
        r += 1;
    }

    if sum > 0.0_f32 {
        let isum = 1.0_f32 / sum;
        let mut r = 0;
        while r < size {
            unsafe {
                *dest.add(r as usize) *= isum;
            }
            r += 1;
        }
    }
}
