use std::os::raw::{c_float, c_int};

#[unsafe(no_mangle)]
pub extern "C" fn gaussian_kernel(dest: *mut c_float, size: c_int, radius: c_float) {
    if dest.is_null() || size <= 0 {
        return;
    }

    let sigma: f32 = 1.6;
    let tetha: f32 = 2.25;
    let hsize = size / 2;
    let s2 = 1.0 / (sigma * sigma * tetha).exp();
    let rs = sigma / radius;
    let mut sum: f32 = 0.0;

    let len = (hsize * 2 + 1).max(size) as usize;
    let dest_slice = unsafe { std::slice::from_raw_parts_mut(dest, len) };

    let mut k = 0;
    for r in -hsize..=hsize {
        let x = (r as f32) * rs;
        let mut v = (1.0 / (x * x).exp()) - s2;
        v = if v > 0.0 { v } else { 0.0 };
        dest_slice[k] = v;
        sum += v;
        k += 1;
    }

    if sum > 0.0 {
        let isum = 1.0 / sum;
        for r in 0..(size as usize) {
            dest_slice[r] *= isum;
        }
    }
}
