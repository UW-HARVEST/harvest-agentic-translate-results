use libc::{c_char, c_int, calloc, size_t, strlen};
use std::ptr;

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

/// Base64 encode and return size data in 'src'. The caller must free the
/// returned string.
/// Returns encoded string otherwise NULL
///
/// # Safety
///
/// `src` must either be NULL or point to a valid buffer of at least `size`
/// bytes (or, if `size == 0`, a NUL-terminated C string).
#[no_mangle]
pub unsafe extern "C" fn encode_base64(mut size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }

    if size == 0 {
        size = strlen(src) as c_int;
    }

    let out_len = (size * 4 / 3 + 4) as size_t;
    let out = calloc(std::mem::size_of::<c_char>() as size_t, out_len) as *mut c_char;
    if out.is_null() {
        return ptr::null_mut();
    }

    let mut p = out;

    let mut i: c_int = 0;
    while i < size {
        let b1: u8;
        let mut b2: u8 = 0;
        let mut b3: u8 = 0;
        let b4: u8;
        let b5: u8;
        let b6: u8;
        let b7: u8;

        b1 = *src.offset(i as isize) as u8;

        if i + 1 < size {
            b2 = *src.offset((i + 1) as isize) as u8;
        }

        if i + 2 < size {
            b3 = *src.offset((i + 2) as isize) as u8;
        }

        b4 = b1 >> 2;
        b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        b7 = b3 & 0x3f;

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

        i += 3;
    }

    out
}
