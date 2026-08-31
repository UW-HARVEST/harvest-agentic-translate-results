// Rust translation of c_src/src/driver.c
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

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uchar;

extern "C" {
    // Use the C library's printf so that output ordering/buffering matches
    // the original C implementation exactly (same stdout FILE stream).
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Mirrors the C `static void print_hex(unsigned char *p, int len)`.
///
/// # Safety
/// `p` must point to at least `len` readable bytes.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // "%02x" with an `unsigned char` argument, which is promoted to `int`.
        printf(b"%02x\0".as_ptr() as *const c_char, *p.offset(i as isize) as c_int);
        i += 1;
    }
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// void driver(float x);
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    // print_hex((unsigned char *)&x, sizeof(x));
    let bytes = x.to_ne_bytes();
    unsafe {
        print_hex(bytes.as_ptr() as *const c_uchar, core::mem::size_of::<f32>() as c_int);
    }
}
