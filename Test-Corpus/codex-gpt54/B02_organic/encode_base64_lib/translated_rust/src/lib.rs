use std::ffi::{c_char, c_int};
use std::ptr;

fn encode(u: u8) -> u8 {
    if u < 26 {
        return b'A' + u;
    }
    if u < 52 {
        return b'a' + (u - 26);
    }
    if u < 62 {
        return b'0' + (u - 52);
    }
    if u == 62 {
        return b'+';
    }

    b'/'
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    let mut size = size;
    let mut i: c_int = 0;

    if src.is_null() {
        return ptr::null_mut();
    }

    if size == 0 {
        size = libc::strlen(src) as c_int;
    }

    let alloc_size = size.wrapping_mul(4) / 3 + 4;
    let out = libc::calloc(1, alloc_size as usize) as *mut c_char;
    if out.is_null() {
        return ptr::null_mut();
    }

    let mut p = out;

    while i < size {
        let mut b2: u8 = 0;
        let mut b3: u8 = 0;

        let b1 = *src.add(i as usize) as u8;

        if i + 1 < size {
            b2 = *src.add((i + 1) as usize) as u8;
        }

        if i + 2 < size {
            b3 = *src.add((i + 2) as usize) as u8;
        }

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        *p = encode(b4) as c_char;
        p = p.add(1);
        *p = encode(b5) as c_char;
        p = p.add(1);

        if i + 1 < size {
            *p = encode(b6) as c_char;
        } else {
            *p = b'=' as c_char;
        }
        p = p.add(1);

        if i + 2 < size {
            *p = encode(b7) as c_char;
        } else {
            *p = b'=' as c_char;
        }
        p = p.add(1);

        i += 3;
    }

    out
}
