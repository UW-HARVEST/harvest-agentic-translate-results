// Rust translation of c_src/src/driver.c (+ c_src/include/driver.h).
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

use core::ffi::{c_char, c_float, c_int, c_uchar};

unsafe extern "C" {
    // Variadic C `printf` from the platform libc.  Writing through libc's
    // `stdout` (rather than Rust's `std::io::stdout`) keeps the byte stream and
    // its buffering semantics identical to the original C library, including
    // when the caller interleaves its own C-side output.
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Internal (non-exported) helper, matching the C `static` function: prints
/// each byte as two lowercase hex digits, then a newline.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]) -- `unsigned char` is promoted to `int`.
        unsafe {
            printf(c"%02x".as_ptr(), c_int::from(*p.offset(i as isize)));
        }
        i += 1;
    }
    // printf("\n") -- a single-character format string; the C compiler lowers
    // this to putchar('\n'), which is what the C .so actually calls.
    unsafe {
        putchar(c_int::from(b'\n'));
    }
}

/// `void driver(float x)`
///
/// Copies the raw object representation of `x` into a local byte buffer and
/// dumps it as hex (in memory / target byte order, i.e. little-endian on x86).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(x: c_float) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [c_char; core::mem::size_of::<c_float>()] =
        unsafe { core::mem::transmute(x.to_bits().to_ne_bytes()) };

    // print_hex((unsigned char *)raw, sizeof(raw));
    unsafe {
        print_hex(raw.as_ptr() as *const c_uchar, raw.len() as c_int);
    }
}
