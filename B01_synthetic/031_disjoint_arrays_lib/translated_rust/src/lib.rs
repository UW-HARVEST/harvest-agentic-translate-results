use std::ffi::{c_char, c_int, CStr};

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: c_int) {
    for i in 0..len as usize {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

fn call_fma(data: &[c_int], len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    let n = len as usize;
    let mut out = vec![0i32; n];
    let ones = vec![1i32; n];
    let zeros = vec![0i32; n];

    // C code sets out[0] = 0 then overwrites via fma_array; replicated by vec![0;n]
    fma_array(&mut out, &ones, data, &zeros, len);
    out[n - 1]
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let mut data = [0i32; 100];
    let mut count = 0usize;

    let c_str = unsafe { CStr::from_ptr(input) };
    let s = c_str.to_str().unwrap_or("");
    let mut remaining = s;

    for _ in 0..100 {
        let trimmed = remaining.trim_start();
        if trimmed.is_empty() {
            break;
        }
        // Find end of integer token (digits, optional leading sign)
        let start = trimmed;
        let end = {
            let bytes = start.as_bytes();
            let mut j = 0;
            if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                j += 1;
            }
            let digit_start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j == digit_start {
                break; // no digits found — matches sscanf failure
            }
            j
        };
        let token = &start[..end];
        if let Ok(val) = token.parse::<i32>() {
            data[count] = val;
            count += 1;
            remaining = &start[end..];
        } else {
            break;
        }
    }

    let result = call_fma(&data[..count], count as c_int);
    println!("{}", result);
}
