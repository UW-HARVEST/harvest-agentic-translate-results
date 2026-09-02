//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (as exported by the C shared library):
//!   int wcscat(wchar_t *dst, size_t numElem, const wchar_t *src);
//!
//! The C behaviour is reproduced exactly, including its quirks:
//!   * the return codes 22 (EINVAL) and 34 (ERANGE) are hard-coded literals,
//!   * `dst[0]` is zeroed on the `!src` and overflow paths,
//!   * an unterminated `dst` buffer makes the copy loop run zero times, so the
//!     function truncates `dst` and reports 34 without touching `src`,
//!   * the bounds check is `ptr < dst + numElem`, i.e. the terminating NUL is
//!     allowed to occupy the last element of the buffer.

#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_uint};

/// `wchar_t` on the Linux/glibc targets this library is built for (4 bytes,
/// signed). Declared explicitly so the ABI matches the C compiler's `wchar_t`.
#[cfg(not(windows))]
pub type wchar_t = c_int;

/// On Windows `wchar_t` is a 16-bit unsigned type.
#[cfg(windows)]
pub type wchar_t = u16;

// `size_t` == `usize` on every platform Rust supports.
#[allow(non_camel_case_types)]
type size_t = usize;

const _: () = {
    // Keep the unused import of c_uint meaningful only if needed; guard the
    // assumption that `wchar_t` is 4 bytes on non-Windows targets.
    #[cfg(not(windows))]
    assert!(core::mem::size_of::<wchar_t>() == core::mem::size_of::<c_uint>());
};

/// Appends `src` to the wide string in `dst`, which holds at most `numElem`
/// elements.
///
/// Returns 0 on success, 22 for invalid arguments, 34 when the result does not
/// fit.
///
/// # Safety
///
/// Same contract as the C original: `dst`, when non-null, must point to
/// `numElem` writable `wchar_t`s and `src`, when non-null, must point to a
/// NUL-terminated wide string.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn wcscat(
    dst: *mut wchar_t,
    numElem: size_t,
    src: *const wchar_t,
) -> c_int {
    // `wchar_t *ptr = dst;` happens before any validation in the C source.
    let mut ptr = dst;

    if dst.is_null() || numElem == 0 {
        return 22;
    }

    // One-past-the-end bound; `dst + numElem` in C.
    //
    // `wrapping_add` (not `add`) is deliberate. For very large `numElem` the C
    // expression `dst + numElem` overflows the address space and wraps, which
    // makes `ptr < dst + numElem` false straight away, so both loops fall
    // through and the function returns 34 with `dst[0] = 0`. That is observable,
    // reproducible behaviour of the compiled C (verified against the built .so
    // for numElem = SIZE_MAX, SIZE_MAX-1, SIZE_MAX/2, 2^62, 2^61). `add` would
    // be undefined behaviour here and would let the optimiser assume no wrap;
    // `wrapping_add` performs exactly the same wrapping byte arithmetic the C
    // compiler emits, and is always safe to compute.
    let end = dst.wrapping_add(numElem);

    if src.is_null() {
        unsafe { *dst = 0 };
        return 22;
    }

    // Seek the existing terminator, but never past the end of the buffer.
    while ptr < end && unsafe { *ptr } != 0 {
        ptr = ptr.wrapping_add(1);
    }

    let mut s = src;
    while ptr < end {
        let ch = unsafe { *s };
        unsafe { *ptr = ch };
        ptr = ptr.wrapping_add(1);
        s = s.wrapping_add(1);
        if ch == 0 {
            return 0;
        }
    }

    unsafe { *dst = 0 };
    34
}
