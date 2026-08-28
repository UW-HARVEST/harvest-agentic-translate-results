//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) whose only
//! public export is `wcscat` (declared in `include/lib.h`):
//!
//! ```c
//! int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src);
//! ```
//!
//! There are no namespace/renaming preprocessor macros in the public header, so
//! the final linker symbol is plainly `wcscat`.
//!
//! The behaviour below is a faithful, bug-for-bug reproduction of the C code:
//! no extra bounds checks, no guaranteed NUL termination on truncation, and the
//! exact same order of validation and the exact same return codes (22 == EINVAL,
//! 34 == ERANGE on this platform).

#![allow(non_camel_case_types)]

use core::ffi::c_int;

/// Platform `wchar_t`.
///
/// On the target platform of the C build (Linux/glibc, GCC) `wchar_t` is a
/// 32-bit signed integer (`int`). Windows uses a 16-bit unsigned type.
#[cfg(not(windows))]
pub type wchar_t = i32;
#[cfg(windows)]
pub type wchar_t = u16;

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
pub unsafe extern "C" fn wcscat(dst: *mut wchar_t, num_elem: usize, src: *const wchar_t) -> c_int {
    // `wchar_t *ptr = dst;`
    let mut ptr: *mut wchar_t = dst;

    // `if (!dst || numElem == 0) return 22;`
    if dst.is_null() || num_elem == 0 {
        return 22;
    }

    // `if (!src) { dst[0] = 0; return 22; }`
    if src.is_null() {
        unsafe { *dst = 0 };
        return 22;
    }

    // `dst + numElem` -- computed once, exactly like the C expression. The
    // wrapping form avoids Rust-level UB while producing the same address the C
    // compiler would compute.
    let end: *mut wchar_t = dst.wrapping_add(num_elem);

    let mut src_ptr: *const wchar_t = src;

    // `while (ptr < dst + numElem && *ptr != 0) ptr++;`
    while ptr < end && unsafe { *ptr } != 0 {
        ptr = ptr.wrapping_add(1);
    }

    // `while (ptr < dst + numElem) { if ((*ptr++ = *src++) == 0) return 0; }`
    while ptr < end {
        let c = unsafe { *src_ptr };
        src_ptr = src_ptr.wrapping_add(1);
        unsafe { *ptr = c };
        ptr = ptr.wrapping_add(1);
        if c == 0 {
            return 0;
        }
    }

    // `dst[0] = 0; return 34;`
    unsafe { *dst = 0 };
    34
}
