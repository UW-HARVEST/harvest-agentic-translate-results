use std::ffi::{CStr, c_char};
use std::os::raw::c_int;

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int]) {
    let len = out.len();
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
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

    fma_array(&mut out, &ones, data, &zeros);
    out[len - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(in_ptr: *const c_char) {
    if in_ptr.is_null() {
        return;
    }
    let c_str = unsafe { CStr::from_ptr(in_ptr) };
    let bytes = c_str.to_bytes();
    
    let mut data = Vec::new();
    let mut i = 0;
    while data.len() < 100 && i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == 0x0B) {
            i += 1;
        }
        if i == bytes.len() {
            break;
        }
        
        let mut j = i;
        let mut sign: c_int = 1;
        if bytes[j] == b'-' {
            sign = -1;
            j += 1;
        } else if bytes[j] == b'+' {
            j += 1;
        }
        
        let mut has_digits = false;
        let mut val: c_int = 0;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            has_digits = true;
            val = val.wrapping_mul(10).wrapping_add((bytes[j] - b'0') as c_int);
            j += 1;
        }
        
        if !has_digits {
            break;
        }
        
        data.push(val.wrapping_mul(sign));
        i = j;
    }

    let result = call_fma(&data);
    println!("{}", result);
}
