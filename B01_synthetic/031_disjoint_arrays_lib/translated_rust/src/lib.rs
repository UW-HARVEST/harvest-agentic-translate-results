use std::ffi::{c_char, c_int, CStr};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    for i in 0..len as usize {
        unsafe {
            *out.add(i) = (*mul1.add(i)) * (*mul2.add(i)) + (*add.add(i));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_fma(data: *const c_int, len: c_int) -> c_int {
    if len == 0 {
        return 0;
    }
    let len = len as usize;
    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];

    let data_slice = unsafe { std::slice::from_raw_parts(data, len) };
    unsafe {
        fma_array(
            out.as_mut_ptr(),
            ones.as_ptr(),
            data_slice.as_ptr(),
            zeros.as_ptr(),
            len as c_int,
        );
    }
    out[len - 1]
}

/// Parse integers from the start of `s` the way C `sscanf(s, "%d", &val)` does.
fn parse_int(s: &[u8]) -> Option<(c_int, usize)> {
    let mut pos = 0;
    while pos < s.len() && (s[pos] == b' ' || s[pos] == b'\t' || s[pos] == b'\n'
        || s[pos] == b'\r' || s[pos] == b'\x0b' || s[pos] == b'\x0c') {
        pos += 1;
    }
    if pos >= s.len() {
        return None;
    }
    let negative = if s[pos] == b'-' {
        pos += 1;
        true
    } else {
        if s[pos] == b'+' {
            pos += 1;
        }
        false
    };
    if pos >= s.len() || !s[pos].is_ascii_digit() {
        return None;
    }
    let mut val: c_int = 0;
    while pos < s.len() && s[pos].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((s[pos] - b'0') as c_int);
        pos += 1;
    }
    if negative {
        val = val.wrapping_neg();
    }
    Some((val, pos))
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(input: *const c_char) {
    let c_str = unsafe { CStr::from_ptr(input) };
    let bytes = c_str.to_bytes();

    let mut data = [0i32; 100];
    let mut i = 0usize;
    let mut offset = 0usize;

    while i < 100 {
        match parse_int(&bytes[offset..]) {
            Some((val, consumed)) => {
                data[i] = val;
                offset += consumed;
                i += 1;
            }
            None => break,
        }
    }

    let result = unsafe { call_fma(data.as_ptr(), i as c_int) };
    print!("{}\n", result);
}
