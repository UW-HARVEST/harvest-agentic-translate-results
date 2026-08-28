//! Rust translation of `c_src/src/lib.c` (base64 encoder).
//!
//! Behaviour is kept bit-for-bit identical to the C original, including its
//! quirks: no explicit NUL terminator (the zeroed `calloc` buffer provides it),
//! `size == 0` meaning "use strlen(src)", and the fact that the returned
//! pointer must be released with `free()` by the caller.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// Mirrors the `static char encode(unsigned char u)` helper.
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

/// Base64 encode and return `size` data in `src`. The caller must free the
/// returned string.
/// Returns encoded string otherwise NULL
#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    let mut size = size;
    if size == 0 {
        // C: size = strlen((char *)src);  (truncating to int, as in the original)
        size = unsafe { strlen(src) } as c_int;
    }

    // C: calloc(sizeof(char), size * 4 / 3 + 4)
    // Reproduce the signed int arithmetic (including a negative `size`
    // producing a huge size_t after the implicit conversion).
    let alloc_len = size.wrapping_mul(4).wrapping_div(3).wrapping_add(4);
    let out = unsafe { calloc(1, alloc_len as usize) } as *mut c_char;
    if out.is_null() {
        return std::ptr::null_mut();
    }

    // Nothing is written when `size <= 0`; the buffer stays all zeroes.
    if size > 0 {
        let n = size as usize;
        let input = unsafe { std::slice::from_raw_parts(src as *const u8, n) };
        // Number of bytes the loop below writes: 4 per group of (up to) 3.
        let written = n.div_ceil(3) * 4;
        let output = unsafe { std::slice::from_raw_parts_mut(out as *mut u8, written) };

        let mut p = 0usize;
        let mut i = 0usize;
        while i < n {
            let b1: u8 = input[i];
            let b2: u8 = if i + 1 < n { input[i + 1] } else { 0 };
            let b3: u8 = if i + 2 < n { input[i + 2] } else { 0 };

            let b4 = b1 >> 2;
            let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
            let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
            let b7 = b3 & 0x3f;

            output[p] = encode(b4);
            p += 1;
            output[p] = encode(b5);
            p += 1;

            output[p] = if i + 1 < n { encode(b6) } else { b'=' };
            p += 1;

            output[p] = if i + 2 < n { encode(b7) } else { b'=' };
            p += 1;

            i += 3;
        }
    }

    out
}
