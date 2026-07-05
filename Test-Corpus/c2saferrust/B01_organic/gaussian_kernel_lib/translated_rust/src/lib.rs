
extern "C" {
    fn expf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
#[no_mangle]
pub fn gaussian_kernel(dest: &mut [f32], size: i32, radius: f32) {
    let sigma: f32 = 1.6;
    let tetha: f32 = 2.25;
    let hsize: i32 = size / 2;

    let s2 = 1.0f32 / (sigma * sigma * tetha).exp();
    let rs = sigma / radius;

    let mut sum = 0.0f32;

    for (i, k) in dest.iter_mut().take(size as usize).enumerate() {
        let r = i as i32 - hsize;
        let x = r as f32 * rs;
        let v = (1.0f32 / (x * x).exp() - s2).max(0.0);
        *k = v;
        sum += v;
    }

    if sum > 0.0 {
        let isum = 1.0f32 / sum;
        for k in dest.iter_mut().take(size as usize) {
            *k *= isum;
        }
    }
}

