use std::ffi::c_int;

#[link(name = "m")]
unsafe extern "C" {
    fn expf(x: f32) -> f32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    let sigma = 1.6f32;
    let tetha = 2.25f32;
    let hsize = size / 2;

    let s2 = 1.0f32 / unsafe { expf(sigma * sigma * tetha) };
    let rs = sigma / radius;
    let mut k = dest;
    let mut sum = 0.0f32;

    let mut r = -hsize;
    while r <= hsize {
        let x = (r as f32) * rs;
        let v = (1.0f32 / unsafe { expf(x * x) }) - s2;
        let v = if v > 0.0f32 { v } else { 0.0f32 };
        unsafe {
            *k = v;
            k = k.add(1);
        }
        sum += v;
        r += 1;
    }

    if sum > 0.0f32 {
        let isum = 1.0f32 / sum;
        let mut r = 0;
        while r < size {
            unsafe {
                *dest.add(r as usize) *= isum;
            }
            r += 1;
        }
    }
}
