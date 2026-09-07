//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C `libdriver.so`):
//!   * `encode_base64`
//!
//! The translation is deliberately bug-compatible with the original C:
//!   * The output buffer is sized `size * 4 / 3 + 4` using *signed* `int`
//!     arithmetic, then passed to `calloc` where it is converted to `size_t`.
//!     A negative value therefore becomes an enormous allocation request that
//!     fails (yielding NULL), and small negative values can still produce a
//!     successful tiny allocation. This is reproduced exactly.
//!   * A `size` of 0 means "treat `src` as a NUL-terminated string", and the
//!     `strlen` result is truncated to `int`, exactly as the C does.
//!   * The terminating NUL of the result relies on `calloc` zeroing the buffer;
//!     no explicit NUL is written.
//!   * Memory is allocated with libc `calloc` (not the Rust allocator) so the
//!     caller can release it with libc `free`, as the C contract requires.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// Translation of the C file-static `encode()` helper.
///
/// C source:
/// ```c
/// static char encode(unsigned char u)
/// {
///     if (u < 26)  return 'A' + u;
///     if (u < 52)  return 'a' + (u - 26);
///     if (u < 62)  return '0' + (u - 52);
///     if (u == 62) return '+';
///     return '/';
/// }
/// ```
///
/// In C the additions are performed in `int` and the result is truncated back
/// to `char`; for every value this function is actually called with (0..=63)
/// the arithmetic stays in range, and `wrapping_add` reproduces the truncation
/// for any other input.
#[inline]
fn encode(u: u8) -> u8 {
    if u < 26 {
        return b'A'.wrapping_add(u);
    }
    if u < 52 {
        return b'a'.wrapping_add(u.wrapping_sub(26));
    }
    if u < 62 {
        return b'0'.wrapping_add(u.wrapping_sub(52));
    }
    if u == 62 {
        return b'+';
    }

    b'/'
}

/// Base64 encode and return `size` data in `src`. The caller must free the
/// returned string.
/// Returns encoded string otherwise NULL
///
/// # Safety
///
/// `src` must either be NULL or point to at least `size` readable bytes (or,
/// when `size` is 0, to a NUL-terminated string). This mirrors the (unchecked)
/// contract of the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(size: c_int, src: *const c_char) -> *mut c_char {
    let mut size: c_int = size;

    if src.is_null() {
        return std::ptr::null_mut();
    }

    if size == 0 {
        // C: size = strlen((char *)src);  -- size_t truncated to int.
        size = unsafe { strlen(src) } as c_int;
    }

    // C: out = (char *)calloc(sizeof(char), size * 4 / 3 + 4);
    //
    // All of this is signed `int` arithmetic in C. `wrapping_*` matches what
    // the compiler emits in practice; the `as usize` conversion sign-extends,
    // exactly like the implicit int -> size_t conversion at the call site.
    let cap_int: c_int = size.wrapping_mul(4).wrapping_div(3).wrapping_add(4);
    let out = unsafe { calloc(std::mem::size_of::<c_char>(), cap_int as usize) } as *mut c_char;
    if out.is_null() {
        return std::ptr::null_mut();
    }

    if size <= 0 {
        // The C loop (`for (i = 0; i < size; i += 3)`) never runs, so the
        // freshly zeroed buffer is returned untouched.
        return out;
    }

    // From here on `size` is positive. The number of bytes the C loop writes is
    // 4 * ceil(size / 3), which is always <= the allocated capacity computed
    // above, so a safe slice over the allocation covers every write.
    let n = size as usize;
    let src_bytes: &[u8] = unsafe { std::slice::from_raw_parts(src as *const u8, n) };
    let dst: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(out as *mut u8, cap_int as usize) };

    let mut p: usize = 0;
    let mut i: usize = 0;
    while i < n {
        // b1..b7 are `unsigned char` in C; reading a (signed) `char` into an
        // `unsigned char` keeps the bit pattern, which is what `u8` gives us.
        let b1: u8 = src_bytes[i];
        let mut b2: u8 = 0;
        let mut b3: u8 = 0;

        if i + 1 < n {
            b2 = src_bytes[i + 1];
        }

        if i + 2 < n {
            b3 = src_bytes[i + 2];
        }

        let b4: u8 = b1 >> 2;
        let b5: u8 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6: u8 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7: u8 = b3 & 0x3f;

        dst[p] = encode(b4);
        p += 1;
        dst[p] = encode(b5);
        p += 1;

        if i + 1 < n {
            dst[p] = encode(b6);
        } else {
            dst[p] = b'=';
        }
        p += 1;

        if i + 2 < n {
            dst[p] = encode(b7);
        } else {
            dst[p] = b'=';
        }
        p += 1;

        i += 3;
    }

    out
}
