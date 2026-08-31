// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C source is a single function that prints `strcspn(s1, s2)` using
//! `printf("%zu\n", ...)`. We call the platform `printf` directly so that the
//! bytes written, and the stdio buffering/flush semantics around them, match
//! the original library exactly.

use std::ffi::{CStr, c_char, c_int};

unsafe extern "C" {
    /// The C library's `printf`, used so output goes through the same stdio
    /// stream (and buffering) as the original C implementation.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Equivalent of C `strcspn`: the length of the initial segment of `s1`
/// consisting of bytes that do not appear in `s2`.
///
/// The terminating NUL of `s2` is not part of the reject set, matching the C
/// standard behaviour (an empty `s2` yields `strlen(s1)`).
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    let mut reject = [false; 256];
    for &b in s2 {
        reject[b as usize] = true;
    }
    s1.iter().take_while(|&&b| !reject[b as usize]).count()
}

/// See `driver` in `c_src/src/driver.c`.
///
/// # Safety
///
/// `s1` and `s2` must be valid pointers to NUL-terminated C strings, exactly as
/// required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    // SAFETY: the caller guarantees both pointers reference NUL-terminated
    // strings; the C original relies on the same precondition.
    let (a, b) = unsafe { (CStr::from_ptr(s1).to_bytes(), CStr::from_ptr(s2).to_bytes()) };

    let n = strcspn(a, b);

    // SAFETY: the format string is a valid NUL-terminated literal and the
    // single variadic argument is a `usize`, which matches `%zu` (`size_t`).
    unsafe {
        printf(c"%zu\n".as_ptr(), n);
    }
}
