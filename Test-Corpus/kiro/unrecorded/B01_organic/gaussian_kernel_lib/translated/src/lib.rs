use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    let sigma: f32 = 1.6;
    let tetha: f32 = 2.25;
    let hsize = size / 2;
    let s2 = 1.0f32 / (sigma * sigma * tetha).exp();
    let rs = sigma / radius;

    let mut sum = 0.0f32;
    let mut k = dest;
    for r in -hsize..=hsize {
        let x = r as f32 * rs;
        let v = (1.0f32 / (x * x).exp()) - s2;
        let v = if v > 0.0 { v } else { 0.0 };
        unsafe { *k = v };
        sum += v;
        k = unsafe { k.add(1) };
    }

    if sum > 0.0 {
        let isum = 1.0f32 / sum;
        for r in 0..size {
            unsafe { *dest.offset(r as isize) *= isum };
        }
    }
}
