//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (must match `nm -D` of the C shared library exactly):
//!   * `encode_base64`
//!
//! The translation is intentionally literal: the arithmetic, the order of the
//! validation checks, the integer truncation/overflow behaviour and the
//! allocation strategy of the original C are reproduced as-is (bugs included).
//! In particular the buffer is allocated with libc's `calloc` because the C API
//! contract says the caller must `free()` the returned string.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// static char encode(unsigned char u)
///
/// Maps a 6-bit value onto the base64 alphabet. Kept private, exactly like the
/// `static` function in the C source (it is not part of the exported ABI).
fn encode(u: u8) -> c_char {
    if u < 26 {
        // 'A' + u
        return (b'A'.wrapping_add(u)) as c_char;
    }
    if u < 52 {
        // 'a' + (u - 26)
        return (b'a'.wrapping_add(u.wrapping_sub(26))) as c_char;
    }
    if u < 62 {
        // '0' + (u - 52)
        return (b'0'.wrapping_add(u.wrapping_sub(52))) as c_char;
    }
    if u == 62 {
        return b'+' as c_char;
    }

    b'/' as c_char
}

/// Base64 encode and return size data in 'src'. The caller must free the
/// returned string.
/// Returns encoded string otherwise NULL
#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    let mut size: c_int = size;

    if src.is_null() {
        return core::ptr::null_mut();
    }

    if size == 0 {
        // size = strlen((char *)src);  -- size_t truncated to int, as in C.
        size = unsafe { strlen(src) } as c_int;
    }

    // out = (char *)calloc(sizeof(char), size * 4 / 3 + 4);
    //
    // `size * 4 / 3 + 4` is computed in `int` (wrapping, matching the codegen
    // of the original) and then converted to `size_t`, i.e. sign extended. A
    // negative result therefore turns into a huge request that `calloc`
    // rejects, which is exactly what the C code does.
    let nbytes: c_int = size
        .wrapping_mul(4)
        .wrapping_div(3)
        .wrapping_add(4);
    let out = unsafe { calloc(core::mem::size_of::<c_char>(), nbytes as isize as usize) } as *mut c_char;
    if out.is_null() {
        return core::ptr::null_mut();
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

            if i.wrapping_add(1) < size {
                *p = encode(b6);
                p = p.add(1);
            } else {
                *p = b'=' as c_char;
                p = p.add(1);
            }

            if i.wrapping_add(2) < size {
                *p = encode(b7);
                p = p.add(1);
            } else {
                *p = b'=' as c_char;
                p = p.add(1);
            }
        }

        i = i.wrapping_add(3);
    }

    out
}
