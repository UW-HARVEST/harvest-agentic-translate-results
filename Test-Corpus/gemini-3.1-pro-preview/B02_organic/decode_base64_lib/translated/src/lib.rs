use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
}

fn decode(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        return c - b'A';
    }
    if c >= b'a' && c <= b'z' {
        return c - b'a' + 26;
    }
    if c >= b'0' && c <= b'9' {
        return c - b'0' + 52;
    }
    if c == b'+' {
        return 62;
    }
    63
}

fn is_base64(c: u8) -> bool {
    (c >= b'A' && c <= b'Z') ||
    (c >= b'a' && c <= b'z') ||
    (c >= b'0' && c <= b'9') ||
    c == b'+' || c == b'/' || c == b'='
}

#[unsafe(no_mangle)]
pub extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(src) };
    let bytes = c_str.to_bytes();

    if bytes.is_empty() {
        return ptr::null_mut();
    }

    let mut buf = Vec::new();
    for &b in bytes {
        if is_base64(b) {
            buf.push(b);
        }
    }

    let mut dest = Vec::new();
    for chunk in buf.chunks(4) {
        let c1 = *chunk.get(0).unwrap_or(&b'A');
        let c2 = *chunk.get(1).unwrap_or(&b'A');
        let c3 = *chunk.get(2).unwrap_or(&b'A');
        let c4 = *chunk.get(3).unwrap_or(&b'A');

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        dest.push((b1 << 2) | (b2 >> 4));

        if c3 != b'=' {
            dest.push(((b2 & 0xf) << 4) | (b3 >> 2));
        }

        if c4 != b'=' {
            dest.push(((b3 & 0x3) << 6) | b4);
        }
    }

    dest.push(0);

    unsafe {
        let ptr = calloc(dest.len(), 1) as *mut c_char;
        if !ptr.is_null() {
            ptr::copy_nonoverlapping(dest.as_ptr() as *const c_char, ptr, dest.len());
        }
        ptr
    }
}
