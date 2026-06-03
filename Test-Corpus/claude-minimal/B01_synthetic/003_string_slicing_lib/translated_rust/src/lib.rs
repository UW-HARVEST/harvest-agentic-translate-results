#![allow(non_snake_case)]

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
use std::os::raw::{c_char, c_int};

/// Index into a passed string
/// and print the substring indexed by [*start_ptr, *stop_ptr).
/// If there is no start, use 0.
/// If there is no stop, use the end of the string.
///
/// # Safety
///
/// `mystr` must be a valid pointer to a NUL-terminated C string.
/// `start_ptr` and `stop_ptr`, if non-null, must point to valid `c_int`s.
#[no_mangle]
pub unsafe extern "C" fn slice(
    mystr: *mut c_char,
    start_ptr: *mut c_int,
    stop_ptr: *mut c_int,
) -> c_int {
    if mystr.is_null() {
        return 1;
    }

    let cstr = CStr::from_ptr(mystr);
    let bytes = cstr.to_bytes();
    let len = bytes.len() as c_int;

    let start: c_int = if !start_ptr.is_null() {
        let s = *start_ptr;
        if s > len {
            println!("Error: start is off the end of the string!");
            return 1;
        }
        s
    } else {
        0
    };

    let stop: c_int = if !stop_ptr.is_null() {
        let s = *stop_ptr;
        if s > len {
            println!("Error: stop is off the end of the string!");
            return 1;
        }
        if s <= start {
            println!("Error: stop must come after start!");
            return 1;
        }
        s
    } else {
        len
    };

    // char arithmetic: skip ahead `start` characters in the array
    let start_usize = start as usize;
    let stop_usize = stop as usize;
    let slice_bytes = &bytes[start_usize..stop_usize];
    // Print substring followed by newline, matching printf("%.*s\n", ...)
    // Use lossy conversion so non-UTF8 bytes don't panic.
    let s = String::from_utf8_lossy(slice_bytes);
    println!("{}", s);

    0
}
