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

use std::os::raw::c_char;
use std::ffi::CStr;

/// Prints a C-style null-terminated string followed by a newline,
/// equivalent to the C `printLine` function. If the pointer is null,
/// nothing is printed.
///
/// # Safety
/// `line` must either be null or point to a valid null-terminated C string.
pub unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        // SAFETY: caller ensures `line` points to a valid null-terminated string.
        let cstr = CStr::from_ptr(line);
        // Print the bytes lossily as UTF-8 followed by a newline.
        println!("{}", cstr.to_string_lossy());
    }
}

/// Mirrors the C `driver(int data)` function.
///
/// Builds a 100-byte `source` buffer filled with 'A's (with a trailing
/// null), then if `data < 100`, copies `data` bytes from `source` to
/// a `dest` buffer and null-terminates it. Finally, prints `dest`.
#[no_mangle]
pub extern "C" fn driver(data: i32) {
    let mut source: [u8; 100] = [b'A'; 100];
    source[100 - 1] = 0;

    let mut dest: [u8; 100] = [0u8; 100];

    if data < 100 {
        // Match the behavior of strncpy(dest, source, data) followed by
        // dest[data] = '\0'. In C, when data is negative, the value is
        // converted to size_t and is enormous, leading to undefined
        // behavior. We faithfully reproduce only the well-defined case
        // where data is non-negative; negative values are treated as a
        // no-op so we don't induce a Rust panic on out-of-bounds.
        if data >= 0 {
            let n = data as usize;
            // strncpy copies up to n bytes; if a null is encountered in
            // source before n bytes, the rest is filled with nulls. Our
            // source has no embedded nulls within the first 99 bytes,
            // and n <= 99 here, so a straight copy of n bytes is correct.
            if n > 0 {
                dest[..n].copy_from_slice(&source[..n]);
            }
            // dest[data] = '\0';
            if n < dest.len() {
                dest[n] = 0;
            }
        }
    }

    // SAFETY: `dest` is a properly null-terminated buffer of length 100.
    unsafe {
        print_line(dest.as_ptr() as *const c_char);
    }
}
