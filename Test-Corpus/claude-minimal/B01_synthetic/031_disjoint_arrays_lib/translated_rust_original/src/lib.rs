// Rust translation of c_src/src/driver.c

use std::ffi::CStr;
use std::os::raw::c_char;

pub fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32], len: usize) {
    for i in 0..len {
        out[i] = mul1[i].wrapping_mul(mul2[i]).wrapping_add(add[i]);
    }
}

pub fn call_fma(data: &[i32], len: usize) -> i32 {
    if len == 0 {
        return 0;
    }
    let mut out = vec![0i32; len];
    let ones = vec![1i32; len];
    let zeros = vec![0i32; len];

    out[0] = 0;

    fma_array(&mut out, &ones, data, &zeros, len);
    out[len - 1]
}

/// Parse a leading integer from `s`, mimicking the C `sscanf("%d", ...)` behavior:
/// - Skips leading ASCII whitespace
/// - Optionally accepts a leading '+' or '-'
/// - Reads consecutive ASCII digit characters
/// Returns Some((value, bytes_consumed_from_start_of_s)) on success, None on failure.
fn parse_int_prefix(s: &[u8]) -> Option<(i32, usize)> {
    let mut idx = 0;

    // Skip leading whitespace (matches C isspace for ASCII whitespace).
    while idx < s.len() && (s[idx] as char).is_ascii_whitespace() {
        idx += 1;
    }

    let sign_start = idx;
    let mut negative = false;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        negative = s[idx] == b'-';
        idx += 1;
    }

    let digits_start = idx;
    while idx < s.len() && (s[idx] as char).is_ascii_digit() {
        idx += 1;
    }

    if idx == digits_start {
        // No digits found.
        return None;
    }

    // Parse the digits as i32, with C-like wrapping on overflow.
    let mut value: i32 = 0;
    for &b in &s[digits_start..idx] {
        let d = (b - b'0') as i32;
        value = value.wrapping_mul(10).wrapping_add(d);
    }
    if negative {
        value = value.wrapping_neg();
    }

    // bytes consumed = idx (we started from 0).
    let _ = sign_start; // not strictly needed
    Some((value, idx))
}

pub fn driver_str(input: &str) -> i32 {
    let mut data = [0i32; 100];
    let mut count = 0usize;
    let mut bytes = input.as_bytes();

    for slot in data.iter_mut().take(100) {
        match parse_int_prefix(bytes) {
            Some((val, nb)) => {
                *slot = val;
                bytes = &bytes[nb..];
                count += 1;
            }
            None => break,
        }
    }

    call_fma(&data[..count], count)
}

/// C-compatible entry point matching `void driver(const char *in);`
///
/// # Safety
/// `in_ptr` must be a valid pointer to a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn driver(in_ptr: *const c_char) {
    if in_ptr.is_null() {
        println!("0");
        return;
    }
    let cstr = CStr::from_ptr(in_ptr);
    // The C code uses sscanf which operates byte-by-byte; treat input as bytes
    // converted lossily to a string. Since parse_int_prefix only inspects
    // ASCII bytes, lossy conversion is safe for purpose.
    let input = match cstr.to_str() {
        Ok(s) => s.to_owned(),
        Err(_) => String::from_utf8_lossy(cstr.to_bytes()).into_owned(),
    };
    let result = driver_str(&input);
    println!("{}", result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fma_array_basic() {
        let mul1 = [1, 2, 3];
        let mul2 = [4, 5, 6];
        let add = [7, 8, 9];
        let mut out = [0i32; 3];
        fma_array(&mut out, &mul1, &mul2, &add, 3);
        assert_eq!(out, [11, 18, 27]);
    }

    #[test]
    fn call_fma_returns_last_data() {
        let data = [10, 20, 30];
        assert_eq!(call_fma(&data, 3), 30);
    }

    #[test]
    fn call_fma_empty() {
        let data: [i32; 0] = [];
        assert_eq!(call_fma(&data, 0), 0);
    }

    #[test]
    fn driver_str_parses_ints() {
        assert_eq!(driver_str("1 2 3 4 5"), 5);
        assert_eq!(driver_str(" 42 "), 42);
        assert_eq!(driver_str(""), 0);
        assert_eq!(driver_str("-7 -8"), -8);
    }
}
