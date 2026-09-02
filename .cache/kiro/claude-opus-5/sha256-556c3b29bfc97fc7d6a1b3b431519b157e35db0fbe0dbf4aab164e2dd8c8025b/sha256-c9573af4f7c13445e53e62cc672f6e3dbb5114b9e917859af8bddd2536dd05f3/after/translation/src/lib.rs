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
/// # Argument read order
///
/// The reject set `s2` is consumed *in full, and first*, before a single byte of
/// `s1` is read. That ordering is not incidental — it is required to match the
/// observable behaviour of the glibc `strcspn` that the C library links against,
/// which was confirmed by probing the compiled `libdriver.so`:
///
/// * glibc inspects the reject set before it touches `s1` (the generic
///   implementation tests `reject[0]`/`reject[1]` to pick its fast paths; the
///   x86-64 SSE4.2 implementation tests `*a == 0`). Consequently
///   `driver("", <invalid s2>)` faults in C — an empty `s1` does *not* short
///   circuit the call. A translation that scanned `s1` first would instead
///   print `0`.
/// * glibc consumes the *whole* reject set to build its lookup table / SIMD
///   mask, so an unterminated `s2` faults even when `s1[0]` is already a member
///   of the set. A translation that searched `s2` linearly per `s1` byte would
///   return early instead of faulting.
///
/// Building a 256-entry membership table up front reproduces both properties
/// while returning exactly the same values for all well-defined inputs.
///
/// # Safety
///
/// `s1` and `s2` must both be valid pointers to NUL-terminated byte strings,
/// exactly as C's `strcspn` requires.
unsafe fn strcspn(s1: *const c_char, s2: *const c_char) -> usize {
    // Membership table for the reject set. Indexed by the byte value widened
    // through `u8`, never through the (signed on x86-64) `c_char`, so bytes
    // 0x80..=0xFF are handled the same way glibc handles them.
    let mut reject = [false; 256];

    // Consume all of `s2` first — see "Argument read order" above.
    let mut j: usize = 0;
    loop {
        // SAFETY: `s2` is a NUL-terminated string and we stop advancing as soon
        // as we observe the NUL, so `s2 + j` stays in bounds.
        let r = unsafe { *s2.add(j) };
        if r == 0 {
            // The terminating NUL is not a member of the reject set.
            break;
        }
        reject[r as u8 as usize] = true;
        j += 1;
    }

    // Then scan `s1`. An empty reject set makes this a plain `strlen`, which is
    // exactly what C's `strcspn` degenerates to in that case.
    let mut i: usize = 0;
    loop {
        // SAFETY: same reasoning as above, for `s1`.
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
