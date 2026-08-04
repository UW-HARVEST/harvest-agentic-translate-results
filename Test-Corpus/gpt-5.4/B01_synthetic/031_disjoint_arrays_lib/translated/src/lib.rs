use std::ffi::{c_char, CStr};
use std::os::raw::c_int;

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int]) {
    for i in 0..out.len() {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[c_int]) -> c_int {
    let len = data.len();
    if len == 0 {
        return 0;
    }

    let mut out = vec![0; len];
    let ones = vec![1; len];
    let zeros = vec![0; len];

    out[0] = 0;
    fma_array(&mut out, &ones, data, &zeros);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_: *const c_char) {
    if in_.is_null() {
        println!("0");
        return;
    }

    let input = unsafe { CStr::from_ptr(in_) };
    let text = input.to_string_lossy();

    let mut data: Vec<c_int> = Vec::with_capacity(100);
    for token in text.split_whitespace().take(100) {
        match token.parse::<c_int>() {
            Ok(value) => data.push(value),
            Err(_) => break,
        }
    }

    let result = call_fma(&data);
    println!("{}", result);
}
