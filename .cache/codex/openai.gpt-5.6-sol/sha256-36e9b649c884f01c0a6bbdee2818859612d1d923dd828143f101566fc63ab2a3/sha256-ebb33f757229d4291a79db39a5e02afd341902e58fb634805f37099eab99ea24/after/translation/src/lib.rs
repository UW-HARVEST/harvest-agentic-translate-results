use std::ffi::{c_float, c_int};

#[link(name = "m")]
unsafe extern "C" {
    fn expf(value: c_float) -> c_float;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut c_float, size: c_int, radius: c_float) {
    let sigma = 1.6_f32;
    let tetha = 2.25_f32;
    let s2 = 1.0_f32 / unsafe { expf(sigma * sigma * tetha) };
    let rs = sigma / radius;
    let hsize = size / 2;
    let mut r = -hsize;
    let mut k = dest;
    let mut sum = 0.0_f32;

    while r <= hsize {
        let x = r as c_float * rs;
        let mut value = (1.0_f32 / unsafe { expf(x * x) }) - s2;
        value = if value > 0.0 { value } else { 0.0 };
        unsafe {
            *k = value;
            k = k.add(1);
        }
        sum += value;
        r += 1;
    }

    if sum > 0.0 {
        let inverse_sum = 1.0_f32 / sum;
        let mut r = 0;
        while r < size {
            unsafe {
                *dest.add(r as usize) *= inverse_sum;
            }
            r += 1;
        }
    }
}
