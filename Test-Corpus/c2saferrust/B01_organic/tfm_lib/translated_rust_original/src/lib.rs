
extern "C" {
    fn sqrtf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
#[no_mangle]
pub fn tfm(dest: &mut [f32], src: &[f32], count: i32) {
    let count = count.max(0) as usize;

    for i in 0..count {
        let src_base = i * 3;
        let dest_base = i * 2;

        let a = src[src_base];
        let b = src[src_base + 1];
        let c = src[src_base + 2];

        if a < b {
            let dx2 = a;
            let dy2 = b;
            let dxy = c;
            let sqd = dy2 * dy2 - 2.0f32 * dx2 * dy2 + dx2 * dx2 + 4.0f32 * dxy * dxy;
            let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0).sqrt());
            dest[dest_base] = dx2 - lambda;
            dest[dest_base + 1] = dxy;
        } else {
            let dy2 = a;
            let dx2 = b;
            let dxy = c;
            let sqd = dy2 * dy2 - 2.0f32 * dx2 * dy2 + dx2 * dx2 + 4.0f32 * dxy * dxy;
            let lambda = 0.5f32 * (dy2 + dx2 + sqd.max(0.0).sqrt());
            dest[dest_base] = dxy;
            dest[dest_base + 1] = dx2 - lambda;
        }
    }
}

