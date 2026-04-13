use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_char as raw_c_char;

fn decode(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        _ => 63,
    }
}

fn is_base64(c: u8) -> bool {
    matches!(c, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=')
}

#[unsafe(no_mangle)]
pub extern "C" fn decode_base64(src: *const raw_c_char) -> *mut raw_c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(src) };
    let src_bytes = c_str.to_bytes();

    if src_bytes.is_empty() {
        return std::ptr::null_mut();
    }

    let filtered: Vec<u8> = src_bytes.iter().copied().filter(|&c| is_base64(c)).collect();

    if filtered.is_empty() {
        let result = CString::new("").unwrap();
        return result.into_raw();
    }

    let mut decoded: Vec<u8> = Vec::with_capacity(filtered.len() * 3 / 4 + 1);

    for chunk in filtered.chunks(4) {
        let c1 = chunk[0];
        let c2 = chunk.get(1).copied().unwrap_or(b'A');
        let c3 = chunk.get(2).copied().unwrap_or(b'A');
        let c4 = chunk.get(3).copied().unwrap_or(b'A');

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        decoded.push((b1 << 2) | (b2 >> 4));

        if c3 != b'=' {
            decoded.push(((b2 & 0xf) << 4) | (b3 >> 2));
        }

        if c4 != b'=' {
            decoded.push(((b3 & 0x3) << 6) | b4);
        }
    }

    match CString::new(decoded) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}