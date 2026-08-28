/*
 * Rust translation of c_src/src/goto.c
 *
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

//! The original C library writes to `stdout` / `stderr` through C `stdio`.
//! To guarantee byte-identical output (identical `%d` / `%s` formatting *and*
//! identical stream buffering / interleaving semantics), this translation
//! drives the very same C `stdio` primitives instead of Rust's `std::io`.

use std::ffi::{c_char, c_int};

/// Opaque stand-in for C's `FILE`.
#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    static mut stderr: *mut FILE;

    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
    fn fclose(stream: *mut FILE) -> c_int;
    fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char;
    fn ferror(stream: *mut FILE) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut FILE, format: *const c_char, ...) -> c_int;
}

/// ```c
/// int forward_goto_example(int x);
/// ```
///
/// Non-`static` in the C source, therefore an exported symbol of the shared
/// library; kept exported here for symbol parity.
#[unsafe(no_mangle)]
pub extern "C" fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        // goto error;
        unsafe { fprintf(stderr, c"Error: negative input\n".as_ptr()) };
        return -1;
    }

    unsafe { printf(c"Processing: %d\n".as_ptr(), x) };
    // Signed overflow is UB in C; reproduce the wrapping behaviour that the
    // compiled C actually exhibits on two's-complement targets.
    x.wrapping_mul(2)
}

/// ```c
/// FILE* open_with_cleanup(const char *filename);
/// ```
///
/// Non-`static` in the C source, therefore an exported symbol of the shared
/// library; kept exported here for symbol parity.
///
/// # Safety
///
/// `filename` must be a valid pointer to a NUL-terminated C string, exactly as
/// required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn open_with_cleanup(filename: *const c_char) -> *mut FILE {
    let fp: *mut FILE = unsafe { fopen(filename, c"r".as_ptr()) };

    // `goto cleanup` on failure, skipping the read loop entirely.
    if !fp.is_null() {
        // char buffer[100];
        let mut buffer = [0 as c_char; 100];

        while !unsafe { fgets(buffer.as_mut_ptr(), buffer.len() as c_int, fp) }.is_null() {
            unsafe { printf(c"%s".as_ptr(), buffer.as_ptr()) };
        }

        if unsafe { ferror(fp) } == 0 {
            return fp;
        }
        // else: goto cleanup;
    }

    // cleanup:
    unsafe {
        fprintf(
            stderr,
            c"Error: opening or processing file %s\n".as_ptr(),
            filename,
        );
        if !fp.is_null() {
            fclose(fp);
        }
    }
    std::ptr::null_mut()
}

/// ```c
/// int driver(int num, const char* filename);
/// ```
///
/// # Safety
///
/// `filename` must be a valid pointer to a NUL-terminated C string, exactly as
/// required by the original C function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        unsafe { printf(c"Goto output: %d\n".as_ptr(), res) };
    }

    let out = unsafe { open_with_cleanup(filename) };
    if out.is_null() {
        return -2;
    } else {
        unsafe { fclose(out) };
    }

    0
}
