use std::os::raw::{c_float, c_int};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut c_float, size: c_int, radius: c_float) {
    let size = size as usize;
    let hsize = size / 2;
    let sigma = 1.6f32;
    let tetha = 2.25f32;
    let s2 = 1.0f32 / (sigma * sigma * tetha).exp();
    let rs = sigma / radius;
    let mut sum = 0.0f32;
    
    let dest_slice = unsafe {
        std::slice::from_raw_parts_mut(dest, size)
    };
    
    for r in -(hsize as isize)..=(hsize as isize) {
        let x = (r as f32) * rs;
        let v = (1.0f32 / (x * x).exp()) - s2;
        let v = v.max(0.0f32);
        let idx = (r + hsize as isize) as usize;
        dest_slice[idx] = v;
        sum += v;
    }
    
    if sum > 0.0f32 {
        let isum = 1.0f32 / sum;
        for r in 0..size {
            dest_slice[r] *= isum;
        }
    }
}
