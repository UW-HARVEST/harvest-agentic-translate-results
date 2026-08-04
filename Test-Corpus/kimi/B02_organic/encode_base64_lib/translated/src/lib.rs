use std::ffi::{c_char, c_int, CStr, CString};
use std::os::raw::c_uchar;

fn encode(u: c_uchar) -> c_char {
    if u < 26 {
        return ('A' as u8 + u) as c_char;
    }
    if u < 52 {
        return ('a' as u8 + (u - 26)) as c_char;
    }
    if u < 62 {
        return ('0' as u8 + (u - 52)) as c_char;
    }
    if u == 62 {
        return '+' as c_char;
    }
    '/' as c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    let size = if size == 0 {
        unsafe { CStr::from_ptr(src).to_bytes().len() as c_int }
    } else {
        size
    };

    let size = size as usize;
    let src_slice = unsafe { std::slice::from_raw_parts(src as *const u8, size) };

    let out_len = size * 4 / 3 + 4;
    let mut out = vec![0u8; out_len];
    let mut p = 0usize;

    for i in (0..size).step_by(3) {
        let b1 = src_slice[i];
        let b2 = if i + 1 < size { src_slice[i + 1] } else { 0 };
        let b3 = if i + 2 < size { src_slice[i + 2] } else { 0 };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        out[p] = encode(b4) as u8;
        p += 1;
        out[p] = encode(b5) as u8;
        p += 1;

        if i + 1 < size {
            out[p] = encode(b6) as u8;
        } else {
            out[p] = b'=';
        }
        p += 1;

        if i + 2 < size {
            out[p] = encode(b7) as u8;
        } else {
            out[p] = b'=';
        }
        p += 1;
    }

    out.truncate(p);
    out.push(0);

    let c_string = match CString::new(out) {
        Ok(cs) => cs,
        Err(_) => return std::ptr::null_mut(),
    };

    c_string.into_raw()
}