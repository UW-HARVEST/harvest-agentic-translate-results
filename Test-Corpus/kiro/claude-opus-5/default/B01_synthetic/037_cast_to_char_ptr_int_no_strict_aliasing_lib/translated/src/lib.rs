// Rust translation of c_src/src/driver.c
//
// Original copyright notice from the C sources:
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

unsafe extern "C" {
    // C `printf` from libc, used so that output goes through the very same
    // stdio stream (and buffering discipline) as the original C library.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Equivalent of the C `static void print_hex(unsigned char *p, int len)`.
///
/// Prints each byte as two lowercase hex digits, then a newline.
fn print_hex(p: &[u8]) {
    for &b in p {
        // "%02x" with the byte promoted to `int`, exactly as in C.
        unsafe {
            printf(c"%02x".as_ptr(), b as c_int);
        }
    }
    unsafe {
        printf(c"\n".as_ptr());
    }
}

/// void driver(int x);
///
/// Copies the raw object representation of `x` into a local buffer and prints
/// it byte by byte in hex (native byte order, matching `memcpy` in the C).
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; size_of::<c_int>()] = x.to_ne_bytes();
    // print_hex((unsigned char *)raw, sizeof(raw));
    print_hex(&raw);
}
