use std::alloc::{alloc_zeroed, Layout};
use std::ffi::{c_char, CStr};
use std::os::raw::c_void;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

fn decode(c: u8) -> u8 {
    if c.is_ascii_uppercase() {
        c - b'A'
    } else if c.is_ascii_lowercase() {
        c - b'a' + 26
    } else if c.is_ascii_digit() {
        c - b'0' + 52
    } else if c == b'+' {
        62
    } else {
        63
    }
}

fn is_base64(c: u8) -> bool {
    c.is_ascii_uppercase()
        || c.is_ascii_lowercase()
        || c.is_ascii_digit()
        || c == b'+'
        || c == b'/'
        || c == b'='
}

#[unsafe(no_mangle)]
pub extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }

    let src_bytes = unsafe { CStr::from_ptr(src) }.to_bytes();
    if src_bytes.is_empty() {
        return ptr::null_mut();
    }

    let src_len = src_bytes.len();
    let alloc_len = src_len + 14;
    let layout = match Layout::array::<u8>(alloc_len) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    let dest = unsafe { alloc_zeroed(layout) } as *mut c_char;
    if dest.is_null() {
        return ptr::null_mut();
    }

    let buf = unsafe { malloc(src_len + 1) } as *mut u8;
    if buf.is_null() {
        return ptr::null_mut();
    }

    let mut filtered_len = 0usize;
    for &b in src_bytes {
        if is_base64(b) {
            unsafe {
                *buf.add(filtered_len) = b;
            }
            filtered_len += 1;
        }
    }

    let mut out = dest as *mut u8;
    let mut k = 0usize;
    while k < filtered_len {
        let c1 = unsafe { *buf.add(k) };
        let c2 = if k + 1 < filtered_len { unsafe { *buf.add(k + 1) } } else { b'A' };
        let c3 = if k + 2 < filtered_len { unsafe { *buf.add(k + 2) } } else { b'A' };
        let c4 = if k + 3 < filtered_len { unsafe { *buf.add(k + 3) } } else { b'A' };

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        unsafe {
            *out = (b1 << 2) | (b2 >> 4);
            out = out.add(1);

            if c3 != b'=' {
                *out = ((b2 & 0x0f) << 4) | (b3 >> 2);
                out = out.add(1);
            }

            if c4 != b'=' {
                *out = ((b3 & 0x03) << 6) | b4;
                out = out.add(1);
            }
        }

        k += 4;
    }

    unsafe {
        free(buf as *mut c_void);
    }

    dest
}
