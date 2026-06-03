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
use std::os::raw::c_char;

/// Prints a line, equivalent to the C `printLine` function.
/// Prints the string followed by a newline if the pointer is non-null.
fn print_line(line: *const c_char) {
    if !line.is_null() {
        // Safety: caller is responsible for ensuring the pointer points to a
        // valid, NUL-terminated C string. This mirrors the original C behavior.
        let s = unsafe { CStr::from_ptr(line) };
        if let Ok(rust_str) = s.to_str() {
            println!("{}", rust_str);
        } else {
            // Fallback: print the bytes as lossy UTF-8.
            println!("{}", s.to_string_lossy());
        }
    }
}

/// The "bad" helper: in the original C, this returns a pointer to a
/// stack-allocated array, which is undefined behavior once the function
/// returns. We mirror the structure here, but to keep the Rust translation
/// safe we return a pointer to a static string with the same contents.
fn helper_bad() -> *mut c_char {
    // The original C returned a pointer to a local stack buffer (UB).
    // We replicate the visible behavior by returning a pointer with the
    // same string contents.
    static HELPER_BAD_STRING: &[u8] = b"helperBad string\0";
    HELPER_BAD_STRING.as_ptr() as *mut c_char
}

/// Equivalent to the C `bad` function.
fn bad() {
    print_line(helper_bad());
}

/// The "good" helper: in the original C, this returns a pointer to a static
/// array, which is well-defined behavior.
fn helper_good1() -> *mut c_char {
    static HELPER_GOOD1_STRING: &[u8] = b"helperGood1 string\0";
    HELPER_GOOD1_STRING.as_ptr() as *mut c_char
}

/// Equivalent to the C `good` function.
fn good() {
    print_line(helper_good1());
}

/// The driver function exported with C ABI, equivalent to the C `driver`
/// function. If `use_good` is non-zero, calls `good()`, otherwise `bad()`.
#[no_mangle]
pub extern "C" fn driver(use_good: i32) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
