use std::os::raw::c_int;
use std::ptr;
use std::slice;

#[unsafe(no_mangle)]
pub extern "C" fn normalize(dest: *mut f32, src: *const f32, size: c_int) {
    if size <= 0 {
        return;
    }

    let len = size as usize;

    let src_slice = unsafe { slice::from_raw_parts(src, len) };
    let sum = src_slice.iter().map(|&x| x * x).sum::<f32>();

    if sum > 0.0 {
        let scale = 1.0f32 / sum.sqrt();
        let dest_slice = unsafe { slice::from_raw_parts_mut(dest, len) };
        for i in 0..len {
            dest_slice[i] = src_slice[i] * scale;
        }
    } else if !ptr::eq(dest as *const f32, src) {
        let dest_slice = unsafe { slice::from_raw_parts_mut(dest, len) };
        dest_slice.fill(0.0);
    }
}
