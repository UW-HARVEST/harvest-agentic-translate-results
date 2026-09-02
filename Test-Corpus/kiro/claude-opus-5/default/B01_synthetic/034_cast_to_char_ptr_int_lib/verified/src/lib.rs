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

use core::ffi::{c_char, c_int, c_uchar};

// The C code writes with printf(3). Go through libc's printf so that the
// output shares the exact same stdout FILE buffer, flush points and
// interleaving behaviour as the original library.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Not exported by the C shared object (it is `static`), so it stays private
/// here as well.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]) -- the unsigned char argument is promoted to int.
        printf(
            b"%02x\0".as_ptr() as *const c_char,
            *p.offset(i as isize) as c_int,
        );
        i += 1;
    }
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// `void driver(int x)`
///
/// Prints the object representation of `x` (4 bytes on the target ABI, in the
/// host's byte order) as lowercase hex, followed by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // print_hex((unsigned char *)&x, sizeof(x));
    let x = x;
    unsafe {
        print_hex(
            &x as *const c_int as *const c_uchar,
            core::mem::size_of::<c_int>() as c_int,
        );
    }
}
