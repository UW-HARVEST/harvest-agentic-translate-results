use std::os::raw::{c_float, c_int};
use std::slice;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize(dest: *mut c_float, src: *const c_float, size: c_int) {
    let size = size as usize;
    let src_slice = unsafe { slice::from_raw_parts(src, size) };
    
    let sum: f32 = src_slice.iter().map(|&x| x * x).sum();
    
    if sum > 0.0 {
        let inv_sqrt = 1.0 / sum.sqrt();
        let dest_slice = unsafe { slice::from_raw_parts_mut(dest, size) };
        for i in 0..size {
            dest_slice[i] = src_slice[i] * inv_sqrt;
        }
    } else if dest != src as *mut c_float {
        let dest_slice = unsafe { slice::from_raw_parts_mut(dest, size) };
        dest_slice.fill(0.0);
    }
}
