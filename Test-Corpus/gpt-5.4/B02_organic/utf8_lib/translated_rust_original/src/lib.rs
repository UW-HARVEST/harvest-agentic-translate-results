use libc::{c_char, c_void, size_t};
use std::ffi::{CStr, CString};

fn valid_1(bytes: &[u8], i: usize) -> bool {
    i < bytes.len() && (bytes[i] & 0x80) == 0
}

fn valid_2(bytes: &[u8], i: usize) -> bool {
    i + 1 < bytes.len()
        && (bytes[i] & 0xE0) == 0xC0
        && bytes[i] >= 0xC2
        && (bytes[i + 1] & 0xC0) == 0x80
}

fn valid_3(bytes: &[u8], i: usize) -> bool {
    i + 2 < bytes.len()
        && (bytes[i] & 0xF0) == 0xE0
        && (bytes[i + 1] & 0xC0) == 0x80
        && (bytes[i + 2] & 0xC0) == 0x80
        && (bytes[i] != 0xE0 || bytes[i + 1] >= 0xA0)
        && (bytes[i] != 0xED || bytes[i + 1] < 0xA0)
        && (bytes[i] != 0xEF || bytes[i + 1] <= 0xBF)
}

fn valid_4(bytes: &[u8], i: usize) -> bool {
    i + 3 < bytes.len()
        && (bytes[i] & 0xF8) == 0xF0
        && bytes[i] <= 0xF4
        && (bytes[i + 1] & 0xC0) == 0x80
        && (bytes[i + 2] & 0xC0) == 0x80
        && (bytes[i + 3] & 0xC0) == 0x80
        && (bytes[i] != 0xF0 || bytes[i + 1] >= 0x90)
        && (bytes[i] != 0xF4 || bytes[i + 1] <= 0x8F)
}

#[unsafe(no_mangle)]
pub extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    assert!(!string.is_null());
    let c_str = unsafe { CStr::from_ptr(string) };
    let bytes = c_str.to_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        if valid_1(bytes, i) {
            i += 1;
        } else if valid_2(bytes, i) {
            i += 2;
        } else if valid_3(bytes, i) {
            i += 3;
        } else if valid_4(bytes, i) {
            i += 4;
        } else {
            return unsafe { string.add(i) };
        }
    }

    unsafe { string.add(i) }
}

#[unsafe(no_mangle)]
pub extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    assert!(!string.is_null());
    let c_str = unsafe { CStr::from_ptr(string) };
    let bytes = c_str.to_bytes();

    let mut valid = 0usize;
    while valid < bytes.len() {
        if valid_1(bytes, valid) {
            valid += 1;
        } else if valid_2(bytes, valid) {
            valid += 2;
        } else if valid_3(bytes, valid) {
            valid += 3;
        } else if valid_4(bytes, valid) {
            valid += 4;
        } else {
            break;
        }
    }

    if valid == bytes.len() {
        return match CString::new(bytes) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        };
    }

    let mut out = Vec::with_capacity(bytes.len() + 1);
    out.extend_from_slice(&bytes[..valid]);

    let mut i = valid;
    while i < bytes.len() {
        if valid_1(bytes, i) {
            out.push(bytes[i]);
            i += 1;
        } else if valid_2(bytes, i) {
            out.extend_from_slice(&bytes[i..i + 2]);
            i += 2;
        } else if valid_3(bytes, i) {
            out.extend_from_slice(&bytes[i..i + 3]);
            i += 3;
        } else if valid_4(bytes, i) {
            out.extend_from_slice(&bytes[i..i + 4]);
            i += 4;
        } else {
            if replacement {
                out.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            }
            i += 1;
        }
    }

    out.push(0);
    let len = out.len();
    let ptr = unsafe { libc::malloc(len as size_t) as *mut c_char };
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        libc::memcpy(ptr as *mut c_void, out.as_ptr() as *const c_void, len as size_t);
    }
    ptr
}
