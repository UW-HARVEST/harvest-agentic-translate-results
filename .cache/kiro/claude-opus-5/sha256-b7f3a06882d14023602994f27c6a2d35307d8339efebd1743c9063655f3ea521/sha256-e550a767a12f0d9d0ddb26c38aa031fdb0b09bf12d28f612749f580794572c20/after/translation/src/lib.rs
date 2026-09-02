// Rust translation of the C library in `c_src/`.
//
// Original C library: Copyright 2025 MIT Lincoln Laboratory (MIT-style license,
// see c_src/include/driver.h for the full notice).
//
// Public ABI reproduced from the C shared library (`nm -D libdriver.so`):
//
//     T driver
//
// That single exported function is the complete public surface of the C library
// (`c_src/include/driver.h` declares only `driver`, and there are no namespace
// macros renaming it, so the linker symbol is plain `driver`).

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    /// The C standard library `printf`. The original C code writes its output
    /// with `printf`, so we call the very same function here: that keeps the
    /// bytes, the `%zu` formatting and the stdio buffering/interleaving
    /// behaviour identical to the C library's.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Reimplementation of C `strcspn`.
///
/// Returns the length of the initial segment of `s1` consisting of bytes that
/// do not appear in `s2`. As in C, the terminating NUL of `s2` is not treated
/// as a member of the reject set, so a byte of `s1` is only ever matched
/// against the non-NUL bytes of `s2`; if no byte of `s1` occurs in `s2` the
/// full length of `s1` is returned.
///
/// # Safety
///
/// `s1` and `s2` must both be valid pointers to NUL-terminated byte strings,
/// exactly as C's `strcspn` requires.
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    let mut i: usize = 0;

    loop {
        // SAFETY: `s1` is a NUL-terminated string and we stop advancing as
        // soon as we observe the NUL, so `s1 + i` stays in bounds.
        let c = unsafe { *s1.add(i) };
        if c == 0 {
            // Reached the end of `s1` without finding a rejected byte.
            return i;
        }

        let mut j: usize = 0;
        loop {
            // SAFETY: same reasoning as above, for `s2`.
            let r = unsafe { *s2.add(j) };
            if r == 0 {
                break;
            }
            if r == c {
                return i;
            }
            j += 1;
        }

        i += 1;
    }
}

/// ```c
/// void driver(const char *s1, const char *s2) {
///     printf("%zu\n", strcspn(s1, s2));
/// }
/// ```
///
/// # Safety
///
/// `s1` and `s2` must be valid NUL-terminated C strings, as required by the
/// original C function (which passes them straight to `strcspn`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    // SAFETY: the caller upholds the same contract the C function requires.
    let n = unsafe { strcspn(s1, s2) };

    // SAFETY: `c"%zu\n"` is a valid NUL-terminated format string and `n` is a
    // `usize` (== C `size_t`), matching the `%zu` conversion specifier.
    unsafe {
        printf(c"%zu\n".as_ptr(), n);
    }
}
