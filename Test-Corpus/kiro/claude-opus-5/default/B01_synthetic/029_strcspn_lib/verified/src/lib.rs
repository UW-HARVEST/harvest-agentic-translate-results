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
/// # Ordering (deliberate)
///
/// This walks the whole reject set `s2` up to its NUL terminator *first*,
/// building a 256-entry membership table, and only then scans `s1`. That
/// ordering is intentional: it reproduces glibc's observable behaviour, in
/// which `strcspn` materialises the reject-set table by walking `s2` before it
/// ever dereferences `s1`. In particular, a NULL `s2` faults immediately (on
/// the very first read of `s2`) regardless of the contents of `s1` — even when
/// `s1` points at an empty string — which a lazy `s1`-first implementation
/// would fail to match.
///
/// # Safety
///
/// `s1` and `s2` must both be valid pointers to NUL-terminated byte strings,
/// exactly as C's `strcspn` requires.
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Build the reject-set membership table by walking `s2` first. This read
    // of `s2` happens unconditionally, before any byte of `s1` is touched, so
    // that a NULL/invalid `s2` faults here exactly as glibc's does.
    let mut reject = [false; 256];
    let mut j: usize = 0;
    loop {
        // SAFETY: `s2` is a NUL-terminated string and we stop advancing as
        // soon as we observe the NUL, so `s2 + j` stays in bounds. `c_char` is
        // `i8` here; cast through `u8` so bytes 0x80..=0xFF map to distinct
        // table slots and never alias slot 0x00.
        let r = unsafe { *s2.add(j) };
        if r == 0 {
            // The NUL terminator of `s2` is not a member of the reject set.
            break;
        }
        reject[r as u8 as usize] = true;
        j += 1;
    }

    // Now scan `s1`, stopping at its NUL or at the first rejected byte.
    let mut i: usize = 0;
    loop {
        // SAFETY: `s1` is a NUL-terminated string and we stop advancing as
        // soon as we observe the NUL, so `s1 + i` stays in bounds.
        let c = unsafe { *s1.add(i) };
        if c == 0 {
            // Reached the end of `s1` without finding a rejected byte.
            return i;
        }
        if reject[c as u8 as usize] {
            return i;
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
