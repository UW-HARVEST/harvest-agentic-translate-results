use std::ffi::{c_char, c_int, CStr};

fn fma_array(out: &mut [c_int], mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: usize) {
    for i in 0..len {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

fn call_fma(data: &[c_int], len: usize) -> c_int {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];
    // Note: C code sets out[0] = 0 then overwrites all via fma_array
    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

/// Parse integers from the start of `s` the way C `sscanf(s, "%d", &val)` does:
/// skip leading whitespace, then parse an optional sign and digits.
/// Returns `(value, bytes_consumed)` or `None` if no integer found.
fn parse_int(s: &[u8]) -> Option<(c_int, usize)> {
    let mut pos = 0;
    // %d skips leading whitespace
    while pos < s.len() && (s[pos] == b' ' || s[pos] == b'\t' || s[pos] == b'\n'
        || s[pos] == b'\r' || s[pos] == b'\x0b' || s[pos] == b'\x0c') {
        pos += 1;
    }
    if pos >= s.len() {
        return None;
    }
    let start = pos;
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
    let _ = start; // total bytes consumed from original pointer is pos
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

    let result = call_fma(&data, i);
    println!("{}", result);
}
