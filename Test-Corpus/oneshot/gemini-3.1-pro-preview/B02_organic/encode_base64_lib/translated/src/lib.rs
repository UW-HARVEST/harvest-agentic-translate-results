use libc::{c_char, c_int, calloc};
use std::ffi::CStr;
use std::ptr;

fn encode(u: u8) -> u8 {
    if u < 26 {
        b'A' + u
    } else if u < 52 {
        b'a' + (u - 26)
    } else if u < 62 {
        b'0' + (u - 52)
    } else if u == 62 {
        b'+'
    } else {
        b'/'
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }

    let actual_size = if size == 0 {
        unsafe { CStr::from_ptr(src) }.to_bytes().len()
    } else if size < 0 {
        return ptr::null_mut();
    } else {
        size as usize
    };

    let out_len = match actual_size.checked_mul(4) {
        Some(v) => v / 3 + 4,
        None => return ptr::null_mut(),
    };

    let out_ptr = unsafe { calloc(out_len, 1) as *mut u8 };
    if out_ptr.is_null() {
        return ptr::null_mut();
    }

    let src_slice = unsafe { std::slice::from_raw_parts(src as *const u8, actual_size) };
    let mut out = Vec::with_capacity(out_len);

    let mut i = 0;
    while i < actual_size {
        let b1 = src_slice[i];
        let b2 = if i + 1 < actual_size { src_slice[i + 1] } else { 0 };
        let b3 = if i + 2 < actual_size { src_slice[i + 2] } else { 0 };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        out.push(encode(b4));
        out.push(encode(b5));

        if i + 1 < actual_size {
            out.push(encode(b6));
        } else {
            out.push(b'=');
        }

        if i + 2 < actual_size {
            out.push(encode(b7));
        } else {
            out.push(b'=');
        }

        i += 3;
    }

    unsafe {
        ptr::copy_nonoverlapping(out.as_ptr(), out_ptr, out.len());
    }

    out_ptr as *mut c_char
}
