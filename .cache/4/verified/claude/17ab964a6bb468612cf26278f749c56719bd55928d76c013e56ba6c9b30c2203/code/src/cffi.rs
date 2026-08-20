/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Minimal bindings to the C runtime functions used by the original C library.
//!
//! The original library allocates every object it hands back to its callers
//! with `malloc()` and it emits all of its diagnostics through the C `stdio`
//! streams.  In order to stay byte-for-byte and ABI compatible (callers are
//! allowed to `free()` the returned pointers, and the diagnostics have to be
//! interleaved with the rest of the process' C `stdio` output in exactly the
//! same way), the translation keeps using the very same C runtime entry points
//! instead of the Rust standard library equivalents.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_ulong, c_void};

/// Opaque stand-in for the C `FILE` type.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    // <stdlib.h>
    pub fn malloc(size: c_ulong) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn atoi(nptr: *const c_char) -> c_int;

    // <string.h>
    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn strtok_r(
        s: *mut c_char,
        delim: *const c_char,
        save_ptr: *mut *mut c_char,
    ) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;

    // <stdio.h>
    pub static mut stderr: *mut FILE;
    pub fn fopen(pathname: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
    pub fn snprintf(str_: *mut c_char, size: c_ulong, format: *const c_char, ...) -> c_int;
    pub fn perror(s: *const c_char);

    // <errno.h>
    fn __errno_location() -> *mut c_int;
}

/// `EINVAL` as defined by Linux' `<errno.h>`.
pub const EINVAL: c_int = 22;

/// `EXIT_SUCCESS` from `<stdlib.h>`.
pub const EXIT_SUCCESS: c_int = 0;

/// `EXIT_FAILURE` from `<stdlib.h>`.
pub const EXIT_FAILURE: c_int = 1;

/// Reads the current value of the C `errno` variable.
#[inline]
pub fn errno() -> c_int {
    unsafe { *__errno_location() }
}

/// Returns the `FILE*` for the C `stderr` stream.
#[inline]
pub fn stderr_stream() -> *mut FILE {
    unsafe { stderr }
}

/// Emulates C's implicit conversion of an `int` to `size_t` (sign extension on
/// LP64 followed by a reinterpretation of the bit pattern as unsigned).
#[inline]
pub fn int_to_size(value: c_int) -> c_ulong {
    value as c_long as c_ulong
}
