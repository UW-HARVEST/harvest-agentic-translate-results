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

use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Mirrors C's `printLine(const char *line)`.
/// Prints the string followed by a newline using libc's printf when the
/// pointer is non-NULL, matching the exact byte output of the C version.
fn print_line(line: *const c_char) {
    if !line.is_null() {
        // "%s\n\0"
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        unsafe {
            printf(fmt, line);
        }
    }
}

/// Mirrors C's `bad()`. The original C reads from an uninitialized
/// `char *data;` local. We reproduce the structure of the bug: an
/// uninitialized local pointer is passed to `print_line`. In Rust we
/// cannot literally read uninitialized memory safely, but we faithfully
/// preserve the buggy behavior pattern using a NULL placeholder; the
/// downstream NULL check in `print_line` ensures defined behavior, just
/// as it does on platforms where the C compiler happens to zero-initialize
/// the stack slot.
fn bad() {
    let data: *const c_char = ptr::null();
    print_line(data);
}

/// Mirrors C's `good()`.
fn good() {
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    print_line(data);
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
