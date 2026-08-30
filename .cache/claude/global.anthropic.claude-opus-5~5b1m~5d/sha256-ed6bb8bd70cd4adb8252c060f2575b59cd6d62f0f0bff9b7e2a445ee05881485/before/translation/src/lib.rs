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

use core::ffi::{c_char, c_int, c_uchar};
use core::mem::size_of;

extern "C" {
    // C stdio printf; used so that output goes through the very same
    // stdout FILE stream (and buffering) that the C library used.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// static void print_hex(unsigned char *p, int len)
fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]);
        unsafe {
            printf(
                b"%02x\0".as_ptr() as *const c_char,
                *p.offset(i as isize) as c_int,
            );
        }
        i += 1;
    }
    // printf("\n");
    unsafe {
        printf(b"\n\0".as_ptr() as *const c_char);
    }
}

/// void driver(int x)
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let mut raw = [0u8; size_of::<c_int>()];
    raw.copy_from_slice(&x.to_ne_bytes());
    // print_hex((unsigned char *)raw, sizeof(raw));
    print_hex(raw.as_ptr() as *const c_uchar, size_of::<c_int>() as c_int);
}
