//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `c_src/include/lib.h`):
//!     int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src);
//!
//! Behaviour is reproduced exactly as written in `c_src/src/lib.c`, including
//! its quirks (e.g. the `dst[0] = 0` truncation stores and the fact that the
//! first scan loop may consume the whole buffer without finding a terminator).

use std::ffi::c_int;

/// `wchar_t` on the Linux/glibc targets this library builds for: a 32-bit
/// signed integer (same as `c_int`).
#[allow(non_camel_case_types)]
pub type wchar_t = c_int;

/// Translation of:
///
/// ```c
/// int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src) {
///     wchar_t *ptr = dst;
///     if (!dst || numElem == 0)
///         return 22;
///     if (!src) {
///         dst[0] = 0;
///         return 22;
///     }
///     while (ptr < dst + numElem && *ptr != 0)
///         ptr++;
///     while (ptr < dst + numElem) {
///         if ((*ptr++ = *src++) == 0)
///             return 0;
///     }
///     dst[0] = 0;
///     return 34;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(
    dst: *mut wchar_t,
    num_elem: usize,
    src: *const wchar_t,
) -> c_int {
    // wchar_t *ptr = dst;
    let mut ptr: *mut wchar_t = dst;

    // if (!dst || numElem == 0) return 22;
    if dst.is_null() || num_elem == 0 {
        return 22;
    }

    // if (!src) { dst[0] = 0; return 22; }
    if src.is_null() {
        unsafe { *dst = 0 };
        return 22;
    }

    // `dst + numElem`, computed the way the C compiler does on this target:
    // a wrapping byte offset of `numElem * sizeof(wchar_t)`.
    let end: *mut wchar_t = dst.wrapping_add(num_elem);

    let mut s: *const wchar_t = src;

    // while (ptr < dst + numElem && *ptr != 0) ptr++;
    while ptr < end && unsafe { *ptr } != 0 {
        ptr = ptr.wrapping_add(1);
    }

    // while (ptr < dst + numElem) { if ((*ptr++ = *src++) == 0) return 0; }
    while ptr < end {
        let c = unsafe { *s };
        s = s.wrapping_add(1);
        unsafe { *ptr = c };
        ptr = ptr.wrapping_add(1);
        if c == 0 {
            return 0;
        }
    }

    // dst[0] = 0;
    unsafe { *dst = 0 };
    // return 34;
    34
}
