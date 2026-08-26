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

//! Bindings to the C runtime facilities used by `c_src/src/lib.c`
//! (`<stdio.h>`, `<stdlib.h>`, `<string.h>`).
//!
//! Output is emitted through the platform's own `printf` rather than through
//! Rust's `std::io::stdout`. That keeps the formatting *and* the stdio stream
//! buffering byte-for-byte identical to the original C library, including
//! interleaving with any `printf` performed by a C caller.

use core::ffi::{c_char, c_int, c_uint, c_void};

unsafe extern "C" {
    /// `int printf(const char *restrict format, ...)`
    pub fn printf(format: *const c_char, ...) -> c_int;
    /// `void *malloc(size_t size)`
    pub fn malloc(size: usize) -> *mut c_void;
    /// `void free(void *ptr)`
    pub fn free(ptr: *mut c_void);
}

/// Emit a format string that takes no arguments.
#[inline]
pub fn print_lit(format: &'static [u8]) {
    debug_assert_eq!(*format.last().unwrap(), 0);
    // SAFETY: `format` is a NUL-terminated literal with no conversion specifiers
    // that consume arguments.
    unsafe { printf(format.as_ptr() as *const c_char) };
}

/// Emit a format string with a single `%d` argument.
#[inline]
pub fn print_i(format: &'static [u8], a: c_int) {
    debug_assert_eq!(*format.last().unwrap(), 0);
    // SAFETY: `format` is a NUL-terminated literal holding exactly one `%d`.
    unsafe { printf(format.as_ptr() as *const c_char, a) };
}

/// Emit a format string with a single `%u`/`%X` argument.
#[inline]
pub fn print_u(format: &'static [u8], a: c_uint) {
    debug_assert_eq!(*format.last().unwrap(), 0);
    // SAFETY: `format` is a NUL-terminated literal holding exactly one
    // unsigned conversion.
    unsafe { printf(format.as_ptr() as *const c_char, a) };
}

/// Emit a format string with a single `%s` argument.
///
/// `s` is forwarded verbatim, so a null pointer reproduces whatever the
/// platform's `printf` does with `%s` and `NULL` (glibc prints `(null)`).
///
/// # Safety
///
/// `s` must be null or point to a NUL-terminated string, exactly as required
/// by the C original.
#[inline]
pub unsafe fn print_s(format: &'static [u8], s: *const c_char) {
    debug_assert_eq!(*format.last().unwrap(), 0);
    // SAFETY: `format` is a NUL-terminated literal holding exactly one `%s`;
    // the caller guarantees `s` is a valid C string or null.
    unsafe { printf(format.as_ptr() as *const c_char, s) };
}

/// Emit a format string taking a `%s` followed by a `%d`.
///
/// # Safety
///
/// `s` must be null or point to a NUL-terminated string.
#[inline]
pub unsafe fn print_s_i(format: &'static [u8], s: *const c_char, a: c_int) {
    debug_assert_eq!(*format.last().unwrap(), 0);
    // SAFETY: `format` is a NUL-terminated literal holding exactly one `%s`
    // then one `%d`; the caller guarantees `s` is a valid C string or null.
    unsafe { printf(format.as_ptr() as *const c_char, s, a) };
}

/// Emit a format string taking four `%d` arguments.
#[inline]
pub fn print_i4(format: &'static [u8], a: c_int, b: c_int, c: c_int, d: c_int) {
    debug_assert_eq!(*format.last().unwrap(), 0);
    // SAFETY: `format` is a NUL-terminated literal holding exactly four `%d`.
    unsafe { printf(format.as_ptr() as *const c_char, a, b, c, d) };
}
