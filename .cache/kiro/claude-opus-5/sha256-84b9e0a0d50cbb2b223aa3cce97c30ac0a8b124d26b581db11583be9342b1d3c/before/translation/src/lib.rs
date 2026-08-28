//! Rust translation of `c_src/src/lib.c`.
//!
//! Provides a `wcscat`-named symbol with the exact semantics (including the
//! quirks) of the original C implementation. The original is *not* the standard
//! `wcscat`: it takes a destination capacity and returns errno-style codes.

use std::ffi::c_int;

/// `wchar_t` on Linux/glibc targets is a 32-bit signed integer, i.e. `c_int`.
#[allow(non_camel_case_types)]
type wchar_t = c_int;

/// Concatenates `src` onto the end of `dst`, bounded by `num_elem` elements.
///
/// Faithful translation of:
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
///
/// Error codes match the C source verbatim: 22 (`EINVAL`) for invalid
/// arguments, 34 (`ERANGE`) when the result does not fit. Note that the
/// original writes the terminator at `dst[0]` (not at the truncation point)
/// on the `ERANGE` path; that behaviour is reproduced as-is.
///
/// # Safety
///
/// Same contract as the C function: `dst`, when non-null, must point to
/// `num_elem` writable `wchar_t`s, and `src`, when non-null, must point to a
/// null-terminated `wchar_t` sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn wcscat(dst: *mut wchar_t, num_elem: usize, src: *const wchar_t) -> c_int {
    unsafe {
        // `wchar_t *ptr = dst;` — assignment happens before any validation.
        let mut ptr = dst;

        if dst.is_null() || num_elem == 0 {
            return 22;
        }

        if src.is_null() {
            *dst = 0;
            return 22;
        }

        // `dst + numElem`: one-past-the-end bound used by both loops.
        let end = dst.add(num_elem);

        // Seek to the existing terminator (or the end of the buffer).
        while ptr < end && *ptr != 0 {
            ptr = ptr.add(1);
        }

        // Copy `src` in, stopping right after the copied terminator.
        let mut s = src;
        while ptr < end {
            let ch = *s;
            s = s.add(1);
            *ptr = ch;
            ptr = ptr.add(1);
            if ch == 0 {
                return 0;
            }
        }

        // Out of room: the C code truncates by clearing the first element.
        *dst = 0;
        34
    }
}
