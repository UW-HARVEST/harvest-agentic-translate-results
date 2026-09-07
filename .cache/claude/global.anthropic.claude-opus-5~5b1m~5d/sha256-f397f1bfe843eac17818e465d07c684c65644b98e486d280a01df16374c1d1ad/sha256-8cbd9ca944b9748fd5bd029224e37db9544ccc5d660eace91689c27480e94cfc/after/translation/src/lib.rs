//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared library):
//!   * `decode_base64`
//!
//! The returned buffer is allocated with the platform `calloc()` so that
//! callers can release it with the platform `free()`, exactly as with the
//! original C implementation.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

/* #define TRUE 1 / #define FALSE 0 */
const TRUE: c_int = 1;
const FALSE: c_int = 0;

/// Decode a base64 character (`static unsigned char decode(char c)`).
fn decode(c: c_char) -> u8 {
    if c >= b'A' as c_char && c <= b'Z' as c_char {
        return (c as i32 - b'A' as i32) as u8;
    }
    if c >= b'a' as c_char && c <= b'z' as c_char {
        return (c as i32 - b'a' as i32 + 26) as u8;
    }
    if c >= b'0' as c_char && c <= b'9' as c_char {
        return (c as i32 - b'0' as i32 + 52) as u8;
    }
    if c == b'+' as c_char {
        return 62;
    }

    63
}

/// Returns TRUE if 'c' is a valid base64 character, otherwise FALSE
/// (`static int is_base64(char c)`).
fn is_base64(c: c_char) -> c_int {
    if (c >= b'A' as c_char && c <= b'Z' as c_char)
        || (c >= b'a' as c_char && c <= b'z' as c_char)
        || (c >= b'0' as c_char && c <= b'9' as c_char)
        || (c == b'+' as c_char)
        || (c == b'/' as c_char)
        || (c == b'=' as c_char)
    {
        return TRUE;
    }
    FALSE
}

/// Decode the base64 encoded string 'src' into a freshly allocated buffer.
/// The dest buffer is NUL terminated.
/// Returns NULL in case of error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    unsafe {
        if !src.is_null() && *src != 0 {
            let dest: *mut c_char;
            let mut p: *mut u8;
            let mut k: c_int;
            /* int k, l = strlen(src) + 1; -- size_t truncated to int */
            let mut l: c_int = strlen(src).wrapping_add(1) as c_int;
            let buf: *mut u8;

            /* The size of the dest will always be less than the source */
            /* calloc(sizeof(char), l + 13); the int argument is sign-extended
             * when converted to size_t. */
            dest = calloc(
                std::mem::size_of::<c_char>(),
                l.wrapping_add(13) as isize as usize,
            ) as *mut c_char;
            if dest.is_null() {
                return ptr::null_mut();
            }

            p = dest as *mut u8;

            buf = malloc(l as isize as usize) as *mut u8;
            if buf.is_null() {
                free(dest as *mut c_void);
                return ptr::null_mut();
            }

            /* Ignore non base64 chars as per the POSIX standard */
            k = 0;
            l = 0;
            while *src.offset(k as isize) != 0 {
                let c = *src.offset(k as isize);
                if is_base64(c) != FALSE {
                    *buf.offset(l as isize) = c as u8;
                    l += 1;
                }
                k += 1;
            }

            k = 0;
            while k < l {
                let mut c2: c_char = b'A' as c_char;
                let mut c3: c_char = b'A' as c_char;
                let mut c4: c_char = b'A' as c_char;

                let c1: c_char = *buf.offset(k as isize) as c_char;

                if k + 1 < l {
                    c2 = *buf.offset((k + 1) as isize) as c_char;
                }

                if k + 2 < l {
                    c3 = *buf.offset((k + 2) as isize) as c_char;
                }

                if k + 3 < l {
                    c4 = *buf.offset((k + 3) as isize) as c_char;
                }

                let b1: u8 = decode(c1);
                let b2: u8 = decode(c2);
                let b3: u8 = decode(c3);
                let b4: u8 = decode(c4);

                *p = ((b1 as u32) << 2 | (b2 as u32) >> 4) as u8;
                p = p.add(1);

                if c3 != b'=' as c_char {
                    *p = (((b2 as u32) & 0xf) << 4 | (b3 as u32) >> 2) as u8;
                    p = p.add(1);
                }

                if c4 != b'=' as c_char {
                    *p = (((b3 as u32) & 0x3) << 6 | (b4 as u32)) as u8;
                    p = p.add(1);
                }

                k += 4;
            }

            free(buf as *mut c_void);

            return dest;
        }
        ptr::null_mut()
    }
}
