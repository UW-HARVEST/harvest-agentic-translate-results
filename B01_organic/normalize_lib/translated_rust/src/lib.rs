use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    let size = size as usize;
    let src_slice = unsafe { std::slice::from_raw_parts(src, size) };
    let dest_slice = unsafe { std::slice::from_raw_parts_mut(dest, size) };

    let sum: f32 = src_slice.iter().map(|x| x * x).sum();
    if sum > 0.0f32 {
        let inv = 1.0f32 / sum.sqrt();
        for i in 0..size {
            dest_slice[i] = src_slice[i] * inv;
        }
    } else if dest.cast_const() != src {
        dest_slice.fill(0.0);
    }
}
