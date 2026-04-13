use std::ffi::{c_char, CStr, CString};
use std::os::raw::c_int;

const REPLACEMENT_INC: usize = 4096;
const REPLACEMENT_CHAR: [u8; 3] = [0xEF, 0xBF, 0xBD];

fn valid_1(x: &[u8]) -> bool {
    (x[0] & 0x80) == 0
}

fn valid_2(x: &[u8]) -> bool {
    (x[0] & 0xE0) == 0xC0
        && x[0] >= 0xC2
        && (x[1] & 0xC0) == 0x80
}

fn valid_3(x: &[u8]) -> bool {
    (x[0] & 0xF0) == 0xE0
        && (x[1] & 0xC0) == 0x80
        && (x[2] & 0xC0) == 0x80
        && (x[0] != 0xE0 || x[1] >= 0xA0)
        && (x[0] != 0xED || x[1] < 0xA0)
        && (x[0] != 0xEF || x[1] <= 0xBF)
}

fn valid_4(x: &[u8]) -> bool {
    (x[0] & 0xF8) == 0xF0
        && x[0] <= 0xF4
        && (x[1] & 0xC0) == 0x80
        && (x[2] & 0xC0) == 0x80
        && (x[3] & 0xC0) == 0x80
        && (x[0] != 0xF0 || x[1] >= 0x90)
        && (x[0] != 0xF4 || x[1] <= 0x8F)
}

fn w_utf8_drop(string: &[u8]) -> usize {
    let mut pos = 0;
    while pos < string.len() && string[pos] != 0 {
        let remaining = &string[pos..];
        if remaining.is_empty() {
            break;
        }
        if valid_1(remaining) {
            pos += 1;
        } else if remaining.len() >= 2 && valid_2(remaining) {
            pos += 2;
        } else if remaining.len() >= 3 && valid_3(remaining) {
            pos += 3;
        } else if remaining.len() >= 4 && valid_4(remaining) {
            pos += 4;
        } else {
            return pos;
        }
    }
    pos
}

#[unsafe(no_mangle)]
pub extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    if string.is_null() {
        return std::ptr::null_mut();
    }
    
    let c_str = unsafe { CStr::from_ptr(string) };
    let bytes = c_str.to_bytes();
    
    let valid_pos = w_utf8_drop(bytes);
    
    if valid_pos >= bytes.len() || bytes[valid_pos] == 0 {
        return unsafe { libc::strdup(string) };
    }
    
    let mut result = Vec::with_capacity(bytes.len() + 1);
    result.extend_from_slice(&bytes[..valid_pos]);
    
    let mut pos = valid_pos;
    let mut repl_remaining: usize = 0;
    
    while pos < bytes.len() && bytes[pos] != 0 {
        let remaining = &bytes[pos..];
        
        if valid_1(remaining) {
            result.push(remaining[0]);
            pos += 1;
        } else if remaining.len() >= 2 && valid_2(remaining) {
            result.push(remaining[0]);
            result.push(remaining[1]);
            pos += 2;
        } else if remaining.len() >= 3 && valid_3(remaining) {
            result.push(remaining[0]);
            result.push(remaining[1]);
            result.push(remaining[2]);
            pos += 3;
        } else if remaining.len() >= 4 && valid_4(remaining) {
            result.push(remaining[0]);
            result.push(remaining[1]);
            result.push(remaining[2]);
            result.push(remaining[3]);
            pos += 4;
        } else {
            if replacement {
                if repl_remaining < 3 {
                    result.reserve(REPLACEMENT_INC);
                    repl_remaining += REPLACEMENT_INC;
                }
                result.extend_from_slice(&REPLACEMENT_CHAR);
                repl_remaining -= 3;
            }
            pos += 1;
        }
    }
    
    result.push(0);
    
    let ptr = unsafe { libc::malloc(result.len()) } as *mut c_char;
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    
    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr() as *const c_char, ptr, result.len());
    }
    
    ptr
}
