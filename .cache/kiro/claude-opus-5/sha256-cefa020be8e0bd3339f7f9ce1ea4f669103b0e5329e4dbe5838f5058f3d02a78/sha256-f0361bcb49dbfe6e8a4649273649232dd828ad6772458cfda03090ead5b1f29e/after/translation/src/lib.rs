// Rust translation of c_src/src/driver.c and c_src/include/driver.h
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
    /// C library `printf`. Used directly (rather than Rust's `print!`) so that
    /// output goes through the very same `stdout` FILE stream, with the same
    /// buffering semantics, as the original C library. This is what makes the
    /// emitted bytes and their flush ordering identical.
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Format string `"%02x"`, NUL terminated.
static FMT_02X: [c_char; 5] = [b'%' as c_char, b'0' as c_char, b'2' as c_char, b'x' as c_char, 0];

/// Format string `"\n"`, NUL terminated.
static FMT_NEWLINE: [c_char; 2] = [b'\n' as c_char, 0];

/// Translation of:
///
/// ```c
/// static void print_hex(unsigned char *p, int len) {
///     for (int i = 0; i < len; i++) {
///         printf("%02x", p[i]);
///     }
///     printf("\n");
/// }
/// ```
///
/// `static` in C, so it is deliberately NOT exported here either.
///
/// # Safety
///
/// `p` must point to at least `len` readable bytes, exactly as required by the
/// original C function.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        // p[i] : unsigned char, promoted to int by the default argument
        // promotions when passed to the variadic printf.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            c_printf(FMT_02X.as_ptr(), byte as c_int);
        }
        i += 1;
    }
    unsafe {
        c_printf(FMT_NEWLINE.as_ptr());
    }
}

/// Translation of:
///
/// ```c
/// void driver(float x) {
///     print_hex((unsigned char *)&x, sizeof(x));
/// }
/// ```
///
/// Prints the object representation of `x` as lowercase, zero padded, two digit
/// hex bytes in memory order, followed by a newline.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_float) {
    // `&x` aliased as `unsigned char *`; `sizeof(float)` bytes are printed.
    unsafe {
        print_hex(
            core::ptr::addr_of!(x) as *const c_uchar,
            core::mem::size_of::<c_float>() as c_int,
        );
    }
}
