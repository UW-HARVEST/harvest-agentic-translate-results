use std::os::raw::c_int;

/// Computes a Gaussian kernel of the given size and radius, writing into `dest`.
///
/// # Safety
///
/// `dest` must point to a writable buffer of at least `size` `f32` elements.
#[no_mangle]
pub unsafe extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    let sigma: f32 = 1.6;
    let tetha: f32 = 2.25;
    let hsize: i32 = size / 2;
    let s2: f32 = 1.0 / (sigma * sigma * tetha).exp();
    let rs: f32 = sigma / radius;

    let size_usize = size as usize;
    let buf = std::slice::from_raw_parts_mut(dest, size_usize);

    let mut sum: f32 = 0.0;
    let mut idx: usize = 0;
    let mut r: i32 = -hsize;
    while r <= hsize {
        let x = r as f32 * rs;
        let mut v = (1.0 / (x * x).exp()) - s2;
        if v <= 0.0 {
            v = 0.0;
        }
        buf[idx] = v;
        sum += v;
        idx += 1;
        r += 1;
    }

    if sum > 0.0 {
        let isum = 1.0 / sum;
        for i in 0..size_usize {
            buf[i] *= isum;
        }
    }
}
