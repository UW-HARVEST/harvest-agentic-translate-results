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

use std::ffi::{c_char, c_int, c_uchar};

unsafe extern "C" {
    /// C `printf` from the platform libc. Used instead of Rust's `std::io::stdout`
    /// so that output goes through the exact same C stdio stream (and buffering)
    /// as the original implementation.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of the C `static void print_hex(unsigned char *p, int len)`.
///
/// Static in C, so it is not exported from the shared object; here it is a
/// private Rust function taking the byte slice the caller would have pointed at.
fn print_hex(p: &[c_uchar], len: c_int) {
    // `for (int i = 0; i < len; i++) printf("%02x", p[i]);`
    let mut i: c_int = 0;
    while i < len {
        // `p[i]` is an `unsigned char`, promoted to `int` for the variadic call.
        let byte: c_uchar = p[i as usize];
        unsafe {
            c_printf(c"%02x".as_ptr(), byte as c_int);
        }
        i += 1;
    }
    // `printf("\n");`
    unsafe {
        c_printf(c"\n".as_ptr());
    }
}

/// Translation of the C `void driver(int x)`.
///
/// The C code reinterprets the storage of the `int` parameter as
/// `unsigned char[sizeof(int)]`, so the output is the native-endian byte
/// representation of `x` (little-endian on x86-64/aarch64).
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // `print_hex((unsigned char *)&x, sizeof(x));`
    let bytes: [c_uchar; size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(&bytes, size_of::<c_int>() as c_int);
}
