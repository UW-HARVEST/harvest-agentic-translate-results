use std::ffi::{c_char, CStr};
use std::ptr;

fn decode(c: c_char) -> u8 {
    let c = c as u8;
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

fn is_base64(c: c_char) -> bool {
    let c = c as u8;
    (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
}

#[unsafe(no_mangle)]
pub extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { CStr::from_ptr(src) };
    let src_bytes = c_str.to_bytes();
    if src_bytes.is_empty() {
        return ptr::null_mut();
    }

    let l = src_bytes.len() + 1;

    // Filter to only base64 characters
    let mut buf: Vec<c_char> = Vec::with_capacity(l);
    for &b in src_bytes {
        if is_base64(b as c_char) {
            buf.push(b as c_char);
        }
    }
    let l = buf.len();

    // Allocate dest with calloc-equivalent (zeroed)
    let mut dest: Vec<u8> = vec![0u8; l + 14];
    let mut p = 0usize;

    let mut k = 0;
    while k < l {
        let c1 = buf[k];
        let c2 = if k + 1 < l { buf[k + 1] } else { b'A' as c_char };
        let c3 = if k + 2 < l { buf[k + 2] } else { b'A' as c_char };
        let c4 = if k + 3 < l { buf[k + 3] } else { b'A' as c_char };

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        dest[p] = (b1 << 2) | (b2 >> 4);
        p += 1;

        if c3 as u8 != b'=' {
            dest[p] = ((b2 & 0xf) << 4) | (b3 >> 2);
            p += 1;
        }

        if c4 as u8 != b'=' {
            dest[p] = ((b3 & 0x3) << 6) | b4;
            p += 1;
        }

        k += 4;
    }

    // NUL terminate (already zeroed, but be explicit)
    dest[p] = 0;

    // Allocate with libc malloc so caller can free() it
    unsafe {
        let ptr = libc::calloc(1, l + 14) as *mut u8;
        if ptr.is_null() {
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping(dest.as_ptr(), ptr, p + 1);
        ptr as *mut c_char
    }
}
