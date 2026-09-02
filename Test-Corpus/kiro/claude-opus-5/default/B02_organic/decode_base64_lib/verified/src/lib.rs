//! Rust translation of c_src/src/lib.c
//!
//! Public ABI (from c_src/include/lib.h):
//!     char *decode_base64(const char *src);
//!
//! The returned buffer is allocated with the C runtime's `calloc` so that
//! callers may release it with `free`, exactly as the original C library did.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_uchar, c_void};

// ---------------------------------------------------------------------------
// C runtime allocator / string helpers.
//
// Declared directly rather than pulling in the `libc` crate so that the crate
// stays dependency free while still using the very same allocator as the C
// original (the caller of `decode_base64` is expected to `free()` the result).
// ---------------------------------------------------------------------------
extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

// #define TRUE    1
const TRUE: c_int = 1;
// #define FALSE   0
const FALSE: c_int = 0;

/// Decode a base64 character.
///
/// Mirrors the `static unsigned char decode(char c)` helper. Note that the C
/// version falls through to `63` for *any* character that is not an upper case
/// letter, lower case letter, digit or `'+'` -- including `'='` and invalid
/// input. That behaviour is preserved verbatim.
#[inline]
fn decode(c: c_char) -> c_uchar {
    if c >= b'A' as c_char && c <= b'Z' as c_char {
        return (c.wrapping_sub(b'A' as c_char)) as c_uchar;
    }
    if c >= b'a' as c_char && c <= b'z' as c_char {
        return (c.wrapping_sub(b'a' as c_char).wrapping_add(26)) as c_uchar;
    }
    if c >= b'0' as c_char && c <= b'9' as c_char {
        return (c.wrapping_sub(b'0' as c_char).wrapping_add(52)) as c_uchar;
    }
    if c == b'+' as c_char {
        return 62;
    }

    63
}

/// Returns TRUE if 'c' is a valid base64 character, otherwise FALSE.
#[inline]
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
/// The buffer is NUL terminated (it is `calloc`ed and over-allocated, exactly
/// like the C original).
///
/// Returns NULL in case of error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if !src.is_null() && *src != 0 {
        // int k, l = strlen(src) + 1;
        let mut l: c_int = (strlen(src) as c_int).wrapping_add(1);

        // The size of the dest will always be less than the source
        // dest = (char *)calloc(sizeof(char), l + 13);
        let dest = calloc(1, (l.wrapping_add(13)) as usize) as *mut c_char;
        if dest.is_null() {
            return std::ptr::null_mut();
        }

        // p = (unsigned char *)dest;
        let mut p = dest as *mut c_uchar;

        // buf = (unsigned char *) malloc(l);
        let buf = malloc(l as usize) as *mut c_uchar;
        if buf.is_null() {
            free(dest as *mut c_void);
            return std::ptr::null_mut();
        }

        // Ignore non base64 chars as per the POSIX standard
        let mut k: c_int = 0;
        l = 0;
        while *src.offset(k as isize) != 0 {
            let c = *src.offset(k as isize);
            if is_base64(c) == TRUE {
                *buf.offset(l as isize) = c as c_uchar;
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

            let b1: c_uchar = decode(c1);
            let b2: c_uchar = decode(c2);
            let b3: c_uchar = decode(c3);
            let b4: c_uchar = decode(c4);

            *p = (b1 << 2) | (b2 >> 4);
            p = p.offset(1);

            if c3 != b'=' as c_char {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.offset(1);
            }

            if c4 != b'=' as c_char {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.offset(1);
            }

            k += 4;
        }

        free(buf as *mut c_void);

        return dest;
    }
    std::ptr::null_mut()
}
