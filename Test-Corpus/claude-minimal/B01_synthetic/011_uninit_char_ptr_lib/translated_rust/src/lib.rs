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

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_int};

/// Prints the C string pointed to by `line`, followed by a newline.
/// If the pointer is null, does nothing.
///
/// # Safety
/// `line` must either be null or point to a valid, NUL-terminated C string.
unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        let cstr = CStr::from_ptr(line);
        match cstr.to_str() {
            Ok(s) => println!("{}", s),
            Err(_) => {
                // Fall back to lossy conversion if the bytes aren't valid UTF-8.
                println!("{}", cstr.to_string_lossy());
            }
        }
    }
}

/// Mirrors the C `bad()` function: declares an uninitialized pointer and
/// passes it to `print_line`. This is undefined behavior, matching the
/// original C source.
#[allow(invalid_value)]
fn bad() {
    // Declare an uninitialized pointer, mirroring `char *data;` in C.
    let data: *const c_char = unsafe { MaybeUninit::uninit().assume_init() };
    unsafe { print_line(data) };
}

/// Mirrors the C `good()` function: assigns a valid string literal to the
/// pointer and prints it.
fn good() {
    // Equivalent to: const char *data = "string";
    let data: *const c_char = b"string\0".as_ptr() as *const c_char;
    unsafe { print_line(data) };
}

/// Public entry point matching the C driver(int useGood) function.
#[no_mangle]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
