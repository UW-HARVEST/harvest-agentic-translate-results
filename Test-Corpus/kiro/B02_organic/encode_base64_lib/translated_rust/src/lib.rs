use std::ffi::{c_char, c_int};

fn encode(u: u8) -> c_char {
    if u < 26 {
        return (b'A' + u) as c_char;
    }
    if u < 52 {
        return (b'a' + (u - 26)) as c_char;
    }
    if u < 62 {
        return (b'0' + (u - 52)) as c_char;
    }
    if u == 62 {
        return b'+' as c_char;
    }
    b'/' as c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn encode_base64(mut size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    if size == 0 {
        size = unsafe { libc::strlen(src as *const _) } as c_int;
    }

    let alloc_size = (size as usize) * 4 / 3 + 4;
    let out = unsafe { libc::calloc(1, alloc_size) } as *mut c_char;
    if out.is_null() {
        return std::ptr::null_mut();
    }

    let src = src as *const u8;
    let mut p = out;
    let mut i: c_int = 0;

    while i < size {
        let b1 = unsafe { *src.offset(i as isize) };
        let b2 = if i + 1 < size {
            unsafe { *src.offset((i + 1) as isize) }
        } else {
            0u8
        };
        let b3 = if i + 2 < size {
            unsafe { *src.offset((i + 2) as isize) }
        } else {
            0u8
        };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        unsafe {
            *p = encode(b4);
            p = p.offset(1);
            *p = encode(b5);
            p = p.offset(1);

            if i + 1 < size {
                *p = encode(b6);
            } else {
                *p = b'=' as c_char;
            }
            p = p.offset(1);

            if i + 2 < size {
                *p = encode(b7);
            } else {
                *p = b'=' as c_char;
            }
            p = p.offset(1);
        }

        i += 3;
    }

    out
}
