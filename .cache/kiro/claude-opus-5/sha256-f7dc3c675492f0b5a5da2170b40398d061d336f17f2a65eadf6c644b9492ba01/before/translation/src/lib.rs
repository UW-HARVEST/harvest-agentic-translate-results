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

use std::ffi::{c_char, c_float, c_int};

unsafe extern "C" {
    // Variadic C `printf`. Used instead of Rust's `println!` so that output goes
    // through the same C stdio buffer the original library used, preserving
    // ordering/flushing behavior when linked alongside other C code.
    #[link_name = "printf"]
    unsafe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of the C `static void print_hex(unsigned char *p, int len)`.
///
/// `printf("%02x", p[i])` promotes the `unsigned char` to `int`, so each byte is
/// passed as a `c_int`.
fn print_hex(p: &[u8], len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            c_printf(c"%02x".as_ptr(), p[i as usize] as c_int);
        }
        i += 1;
    }
    unsafe {
        c_printf(c"\n".as_ptr());
    }
}

/// Translation of `void driver(float x)`.
///
/// The C code copies the raw object representation of `x` into a `char[4]`
/// buffer via `memcpy` and prints those bytes in memory order, i.e. native
/// (little-endian on x86_64/aarch64) byte order.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; core::mem::size_of::<c_float>()] = x.to_bits().to_ne_bytes();

    // print_hex((unsigned char *)raw, sizeof(raw));
    print_hex(&raw, raw.len() as c_int);
}
