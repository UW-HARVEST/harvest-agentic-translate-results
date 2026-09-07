// Rust translation of the C library in c_src/.
//
// Original copyright header from the C sources:
//
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

#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};

// libc declarations. The C library writes its output through the C runtime's
// `stdout`, so we go through the very same `printf` in order to keep the
// byte stream (and its buffering behaviour) identical.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
}

/// Translation of:
/// ```c
/// void printLine (const char * line)
/// {
///     if(line != NULL)
///     {
///         printf("%s\n", line);
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

/// Translation of:
/// ```c
/// void driver(int data)
/// {
///     char source[100];
///     char dest[100] = "";
///     memset(source, 'A', 100-1);
///     source[100-1] = '\0';
///     if (data < 100)
///     {
///         strncpy(dest, source, data);
///         dest[data] = '\0';
///     }
///     printLine(dest);
/// }
/// ```
///
/// The out-of-bounds behaviour of the original (e.g. a negative `data`) is
/// deliberately reproduced rather than fixed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    // `char source[100];` -- uninitialized in C; fully overwritten below.
    let mut source: [c_char; 100] = [0; 100];
    // `char dest[100] = "";` -- zero initialized.
    let mut dest: [c_char; 100] = [0; 100];

    unsafe {
        memset(source.as_mut_ptr() as *mut c_void, b'A' as c_int, 100 - 1);
        source[100 - 1] = 0;

        if data < 100 {
            strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }

        printLine(dest.as_ptr());
    }
}
