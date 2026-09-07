//! Rust translation of `c_src/src/lib.c` (public header: `c_src/include/lib.h`).
//!
//! Public ABI reproduced (matches `nm -D --defined-only` on the C `.so`):
//!   * `encode_base64`
//!
//! The returned buffer is allocated with the C runtime's `calloc`, exactly as
//! in the original, so that callers may release it with `free()`.
//!
//! Quirks of the C implementation that are deliberately preserved:
//!   * `size == 0` is treated as "measure `src` with `strlen`" (and the
//!     `size_t` result is truncated to `int`).
//!   * The allocation length is computed with `int` arithmetic
//!     (`size * 4 / 3 + 4`) and then widened to `size_t` by sign extension,
//!     so negative/overflowing sizes behave just like the C code (e.g. a
//!     negative `size` yields either a `NULL` return from a huge request or a
//!     zero-length allocation, never any output bytes).
//!   * No trailing NUL is written explicitly; it comes from `calloc` zeroing.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// `static char encode(unsigned char u)`
///
/// Not exported (it is `static` in the C translation unit).
#[inline]
fn encode(u: u8) -> c_char {
    if u < 26 {
        return (b'A' as c_int + u as c_int) as c_char;
    }
    if u < 52 {
        return (b'a' as c_int + (u as c_int - 26)) as c_char;
    }
    if u < 62 {
        return (b'0' as c_int + (u as c_int - 52)) as c_char;
    }
    if u == 62 {
        return b'+' as c_char;
    }

    b'/' as c_char
}

/// Base64 encode and return `size` data in `src`. The caller must free the
/// returned string.
/// Returns encoded string otherwise NULL
#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    let mut size: c_int = size;

    if src.is_null() {
        return ptr::null_mut();
    }

    if size == 0 {
        // `size = strlen((char *)src);` -- size_t truncated to int.
        size = unsafe { strlen(src) } as c_int;
    }

    // `calloc(sizeof(char), size * 4 / 3 + 4)`: int arithmetic, then the
    // (possibly negative) int is sign-extended into calloc's size_t argument.
    let n: c_int = size.wrapping_mul(4).wrapping_div(3).wrapping_add(4);
    let out = unsafe { calloc(1, n as isize as usize) } as *mut c_char;
    if out.is_null() {
        return ptr::null_mut();
    }

    let mut p: *mut c_char = out;

    let mut i: c_int = 0;
    while i < size {
        let b1: u8;
        let mut b2: u8 = 0;
        let mut b3: u8 = 0;

        b1 = unsafe { *src.offset(i as isize) } as u8;

        if i.wrapping_add(1) < size {
            b2 = unsafe { *src.offset(i.wrapping_add(1) as isize) } as u8;
        }

        if i.wrapping_add(2) < size {
            b3 = unsafe { *src.offset(i.wrapping_add(2) as isize) } as u8;
        }

        let b4: u8 = b1 >> 2;
        let b5: u8 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6: u8 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7: u8 = b3 & 0x3f;

        unsafe {
            *p = encode(b4);
            p = p.add(1);
            *p = encode(b5);
            p = p.add(1);
        }

        if i.wrapping_add(1) < size {
            unsafe {
                *p = encode(b6);
                p = p.add(1);
            }
        } else {
            unsafe {
                *p = b'=' as c_char;
                p = p.add(1);
            }
        }

        if i.wrapping_add(2) < size {
            unsafe {
                *p = encode(b7);
                p = p.add(1);
            }
        } else {
            unsafe {
                *p = b'=' as c_char;
                p = p.add(1);
            }
        }

        i = i.wrapping_add(3);
    }

    out
}
