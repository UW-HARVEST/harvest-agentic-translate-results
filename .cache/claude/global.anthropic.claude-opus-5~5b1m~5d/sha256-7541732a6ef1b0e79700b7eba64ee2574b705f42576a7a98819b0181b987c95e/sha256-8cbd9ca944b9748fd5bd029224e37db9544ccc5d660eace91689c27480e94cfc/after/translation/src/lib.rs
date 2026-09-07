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
use std::ffi::c_uchar;

extern "C" {
    // Use the C library's printf so that stdout buffering / interleaving with
    // any other C code in the process is byte-for-byte identical to the
    // original library.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Not exported by the C shared library (it is `static`), so it stays private
/// here as well.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]); -- the unsigned char argument is promoted to int.
        printf(
            b"%02x\0".as_ptr() as *const c_char,
            *p.offset(i as isize) as c_int,
        );
        i += 1;
    }
    // printf("\n");
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// `void driver(int x)`
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    unsafe {
        print_hex(
            &x as *const c_int as *const c_uchar,
            core::mem::size_of::<c_int>() as c_int,
        );
    }
}
