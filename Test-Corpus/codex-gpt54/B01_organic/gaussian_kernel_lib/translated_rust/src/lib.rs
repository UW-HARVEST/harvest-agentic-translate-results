use std::ffi::{c_float, c_int};

#[link(name = "m")]
unsafe extern "C" {
    fn expf(x: c_float) -> c_float;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut c_float, size: c_int, radius: c_float) {
    let rs: c_float;
    let s2: c_float;
    let mut sum: c_float;
    let sigma: c_float = 1.6_f32;
    let tetha: c_float = 2.25_f32;
    let hsize: c_int = size / 2;

    s2 = 1.0_f32 / unsafe { expf(sigma * sigma * tetha) };
    rs = sigma / radius;

    let mut k = dest;
    sum = 0.0_f32;

    let mut r: c_int = -hsize;
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
        let mut r: c_int = 0;
        while r < size {
            unsafe {
                let idx = r as usize;
                *dest.add(idx) *= isum;
            }
            r += 1;
        }
    }
}
