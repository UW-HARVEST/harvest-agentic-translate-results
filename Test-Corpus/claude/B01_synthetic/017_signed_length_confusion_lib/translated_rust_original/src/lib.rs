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

use core::ffi::{c_char, c_int};

// Bind to libc's printf, strncpy, memset to faithfully reproduce the C
// behavior, including any quirks/UB present in the original program (e.g.,
// when `data` is negative, strncpy's size_t argument receives a huge value).
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn strncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn memset(s: *mut core::ffi::c_void, c: c_int, n: usize) -> *mut core::ffi::c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // printf("%s\n", line);
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe { printf(fmt, line) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    let mut source: [c_char; 100] = [0; 100];
    let mut dest: [c_char; 100] = [0; 100];

    // memset(source, 'A', 100-1);
    unsafe {
        memset(
            source.as_mut_ptr() as *mut core::ffi::c_void,
            b'A' as c_int,
            100 - 1,
        );
    }
    // source[100-1] = '\0';
    source[100 - 1] = 0;

    if data < 100 {
        // strncpy(dest, source, data);
        // Preserve original behavior: `data` is `int`, but strncpy takes size_t.
        // Negative values become very large size_t, mirroring the C UB.
        unsafe {
            strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
        }
        // dest[data] = '\0';
        // Match the unchecked indexing in C (would be UB for data < 0 or > 99).
        unsafe {
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    unsafe { printLine(dest.as_ptr()) };
}
