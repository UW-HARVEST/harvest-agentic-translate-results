use std::os::raw::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn gaussian_kernel(dest: *mut f32, size: c_int, radius: f32) {
    if dest.is_null() || size <= 0 {
        return;
    }

    let size_usize = size as usize;
    let hsize = size / 2;
    let sigma = 1.6f32;
    let tetha = 2.25f32;
    let s2 = 1.0f32 / (sigma * sigma * tetha).exp();
    let rs = sigma / radius;
    let dest_slice = unsafe { std::slice::from_raw_parts_mut(dest, size_usize) };

    let mut sum = 0.0f32;
    for (i, r) in (-hsize..=hsize).enumerate() {
        if i >= dest_slice.len() {
            break;
        }
        let x = r as f32 * rs;
        let mut v = (1.0f32 / (x * x).exp()) - s2;
        v = v.max(0.0f32);
        dest_slice[i] = v;
        sum += v;
    }

    if sum > 0.0f32 {
        let isum = 1.0f32 / sum;
        for v in dest_slice.iter_mut() {
            *v *= isum;
        }
    }
}
