use std::os::raw::{c_int, c_void};
use std::slice;

fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn inner(out: &mut [i32], len: usize) {
    fma_array(out, out, out, out, len);
    for i in 0..len {
        println!("{}", out[i]);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: *const c_int, len: c_int) {
    let len = len as usize;
    let data_slice = unsafe {
        slice::from_raw_parts(data, len)
    };
    let mut out: Vec<i32> = data_slice.to_vec();
    inner(&mut out, len);
}