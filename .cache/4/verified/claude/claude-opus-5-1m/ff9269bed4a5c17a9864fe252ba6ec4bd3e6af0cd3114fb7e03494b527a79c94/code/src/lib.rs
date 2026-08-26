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

//! Rust translation of `c_src/src/driver.c`.
//!
//! The C library exports exactly one public symbol, `driver`, which prints the
//! raw in-memory bytes of its `int` argument as lowercase hex (two digits per
//! byte, no separators) followed by a newline.
//!
//! Output is emitted through the C library's `printf` (i.e. the process's
//! `stdout` `FILE*`) so that buffering, ordering and flushing semantics are
//! byte-for-byte identical to the original C implementation.

#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uchar;

extern "C" {
    /// The C standard library `printf`. Declared directly (rather than pulling
    /// in a crate) so we write to the very same `stdout` stream the C code did.
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Internal (non-exported) helper: identical to the `static` C function.
/// Iterates `len` bytes starting at `p`, printing each as `%02x`, then a
/// trailing newline.
///
/// # Safety
///
/// `p` must be valid for reads of `len` bytes when `len > 0`. Exactly as in the
/// C original, a negative or zero `len` simply skips the loop.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    // `for (int i = 0; i < len; i++)` -- no iterations when len <= 0.
    let mut i: c_int = 0;
    while i < len {
        // `printf("%02x", p[i]);`
        // The C `unsigned char` argument is promoted to `int` by the default
        // argument promotions, so pass it as a `c_int`.
        let byte = *p.offset(i as isize);
        printf(b"%02x\0".as_ptr() as *const c_char, byte as c_int);
        i += 1;
    }
    // `printf("\n");`
    printf(b"\n\0".as_ptr() as *const c_char);
}

/// `void driver(int x)`
///
/// Prints the object representation of `x` (`sizeof(int)` == 4 bytes) in memory
/// order -- little-endian on all supported targets -- as hex.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // `print_hex((unsigned char *)&x, sizeof(x));`
    //
    // `x` is a local copy of the parameter, exactly as in C, so taking its
    // address and reading `sizeof(int)` bytes is well defined here.
    let x = x;
    unsafe {
        print_hex(
            &x as *const c_int as *const c_uchar,
            core::mem::size_of::<c_int>() as c_int,
        );
    }
}
