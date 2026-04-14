use std::ffi::{CStr, CString, c_char};
use std::os::raw::c_int;
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

    let input = unsafe {
        if size == 0 {
            CStr::from_ptr(src).to_bytes().to_vec()
        } else if size < 0 {
            return ptr::null_mut();
        } else {
            std::slice::from_raw_parts(src as *const u8, size as usize).to_vec()
        }
    };

    let len = input.len();
    let mut out = Vec::with_capacity(len * 4 / 3 + 4);

    let mut i = 0;
    while i < len {
        let b1 = input[i];
        let b2 = if i + 1 < len { input[i + 1] } else { 0 };
        let b3 = if i + 2 < len { input[i + 2] } else { 0 };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        out.push(encode(b4));
        out.push(encode(b5));

        if i + 1 < len {
            out.push(encode(b6));
        } else {
            out.push(b'=');
        }

        if i + 2 < len {
            out.push(encode(b7));
        } else {
            out.push(b'=');
        }

        i += 3;
    }

    match CString::new(out) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}
