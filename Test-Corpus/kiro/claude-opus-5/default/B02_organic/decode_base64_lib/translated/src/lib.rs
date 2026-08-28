//! Rust translation of `c_src/src/lib.c` (base64 decoder).
//!
//! The behaviour — including quirks of the original implementation — is
//! reproduced exactly, so that the same input yields byte-identical output.
//!
//! The returned buffer is allocated with the C allocator (`calloc`) because the
//! caller is expected to release it with `free()`.

use std::ffi::{c_char, c_int, c_uchar, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

const TRUE: c_int = 1;
const FALSE: c_int = 0;

/// Decode a base64 character.
fn decode(c: u8) -> c_uchar {
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

/// Returns TRUE if `c` is a valid base64 character, otherwise FALSE.
fn is_base64(c: u8) -> c_int {
    if (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || (c == b'+')
        || (c == b'/')
        || (c == b'=')
    {
        return TRUE;
    }
    FALSE
}

/// Decode the base64 encoded string `src` into a freshly allocated, NUL
/// terminated buffer.
///
/// Returns NULL in case of error.
///
/// # Safety
///
/// `src` must either be NULL or point to a NUL terminated string. The returned
/// pointer, when non-NULL, must be released with `free()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() || unsafe { *src } == 0 {
        return std::ptr::null_mut();
    }

    // strlen(src) + 1
    let mut len: usize = 0;
    while unsafe { *src.add(len) } != 0 {
        len += 1;
    }
    let l0 = len + 1;

    /* The size of the dest will always be less than the source */
    let dest = unsafe { calloc(std::mem::size_of::<c_char>(), l0 + 13) } as *mut c_char;
    if dest.is_null() {
        return std::ptr::null_mut();
    }

    let mut p = dest as *mut c_uchar;

    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve_exact(l0).is_err() {
        unsafe { free(dest as *mut c_void) };
        return std::ptr::null_mut();
    }

    // Source bytes as a slice (excluding the NUL terminator).
    let input = unsafe { std::slice::from_raw_parts(src as *const u8, len) };

    /* Ignore non base64 chars as per the POSIX standard */
    for &c in input {
        if is_base64(c) == TRUE {
            buf.push(c);
        }
    }

    let l = buf.len();

    let mut k = 0usize;
    while k < l {
        let c1: u8;
        let mut c2: u8 = b'A';
        let mut c3: u8 = b'A';
        let mut c4: u8 = b'A';

        c1 = buf[k];

        if k + 1 < l {
            c2 = buf[k + 1];
        }

        if k + 2 < l {
            c3 = buf[k + 2];
        }

        if k + 3 < l {
            c4 = buf[k + 3];
        }

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        unsafe {
            *p = (b1 << 2) | (b2 >> 4);
            p = p.add(1);

            if c3 != b'=' {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.add(1);
            }

            if c4 != b'=' {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.add(1);
            }
        }

        k += 4;
    }

    dest
}
