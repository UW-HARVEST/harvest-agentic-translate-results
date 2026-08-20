//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (matches `nm -D` of the C shared library):
//!   * `decode_base64`
//!
//! The returned buffer is allocated with the platform allocator (`calloc`) so
//! that callers can release it with `free()`, exactly like the C original.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

const TRUE: c_int = 1;
const FALSE: c_int = 0;

/* Decode a base64 character */
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

/* Returns TRUE if 'c' is a valid base64 character, otherwise FALSE */
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

/* Decode the base64 encoded string 'src' into a freshly allocated buffer.
 * The buffer is NUL terminated (it is zero filled on allocation).
 * Returns NULL in case of error.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if !src.is_null() && unsafe { *src } != 0 {
        // int k, l = strlen(src) + 1;
        let mut l: c_int = (unsafe { strlen(src) } as c_int).wrapping_add(1);

        /* The size of the dest will always be less than the source */
        let dest = unsafe { calloc(1, l.wrapping_add(13) as isize as usize) } as *mut c_char;
        if dest.is_null() {
            return std::ptr::null_mut();
        }

        let mut p = dest as *mut u8;

        let buf = unsafe { malloc(l as isize as usize) } as *mut u8;
        if buf.is_null() {
            unsafe { free(dest as *mut c_void) };
            return std::ptr::null_mut();
        }

        /* Ignore non base64 chars as per the POSIX standard */
        let mut k: c_int = 0;
        l = 0;
        while unsafe { *src.offset(k as isize) } != 0 {
            let c = unsafe { *src.offset(k as isize) };
            if is_base64(c) != FALSE {
                unsafe { *buf.offset(l as isize) = c as u8 };
                l += 1;
            }
            k += 1;
        }

        k = 0;
        while k < l {
            let mut c2: c_char = b'A' as c_char;
            let mut c3: c_char = b'A' as c_char;
            let mut c4: c_char = b'A' as c_char;

            let c1: c_char = unsafe { *buf.offset(k as isize) } as c_char;

            if k + 1 < l {
                c2 = unsafe { *buf.offset((k + 1) as isize) } as c_char;
            }

            if k + 2 < l {
                c3 = unsafe { *buf.offset((k + 2) as isize) } as c_char;
            }

            if k + 3 < l {
                c4 = unsafe { *buf.offset((k + 3) as isize) } as c_char;
            }

            let b1 = decode(c1);
            let b2 = decode(c2);
            let b3 = decode(c3);
            let b4 = decode(c4);

            unsafe {
                *p = ((b1 as i32) << 2 | (b2 as i32) >> 4) as u8;
                p = p.add(1);
            }

            if c3 != b'=' as c_char {
                unsafe {
                    *p = (((b2 as i32 & 0xf) << 4) | ((b3 as i32) >> 2)) as u8;
                    p = p.add(1);
                }
            }

            if c4 != b'=' as c_char {
                unsafe {
                    *p = (((b3 as i32 & 0x3) << 6) | b4 as i32) as u8;
                    p = p.add(1);
                }
            }

            k += 4;
        }

        unsafe { free(buf as *mut c_void) };

        return dest;
    }
    std::ptr::null_mut()
}
