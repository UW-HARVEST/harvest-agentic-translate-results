use std::ffi::{c_char, c_int};
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
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }
    if *src == 0 {
        return ptr::null_mut();
    }

    let mut l: c_int = (libc::strlen(src) + 1) as c_int;

    let dest = libc::calloc(std::mem::size_of::<c_char>(), l.wrapping_add(13) as usize) as *mut c_char;
    if dest.is_null() {
        return ptr::null_mut();
    }

    let mut p = dest as *mut u8;

    let buf = libc::malloc(l as usize) as *mut c_char;
    if buf.is_null() {
        libc::free(dest.cast());
        return ptr::null_mut();
    }

    let mut k: c_int = 0;
    l = 0;
    while *src.add(k as usize) != 0 {
        let ch = *src.add(k as usize);
        if is_base64(ch) {
            *buf.add(l as usize) = ch;
            l += 1;
        }
        k += 1;
    }

    k = 0;
    while k < l {
        let c1;
        let mut c2 = b'A' as c_char;
        let mut c3 = b'A' as c_char;
        let mut c4 = b'A' as c_char;

        c1 = *buf.add(k as usize);

        if k + 1 < l {
            c2 = *buf.add((k + 1) as usize);
        }

        if k + 2 < l {
            c3 = *buf.add((k + 2) as usize);
        }

        if k + 3 < l {
            c4 = *buf.add((k + 3) as usize);
        }

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        *p = (b1 << 2) | (b2 >> 4);
        p = p.add(1);

        if c3 != b'=' as c_char {
            *p = ((b2 & 0x0f) << 4) | (b3 >> 2);
            p = p.add(1);
        }

        if c4 != b'=' as c_char {
            *p = ((b3 & 0x03) << 6) | b4;
            p = p.add(1);
        }

        k += 4;
    }

    libc::free(buf.cast());

    dest
}
