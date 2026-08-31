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

unsafe extern "C" {
    // Use the C runtime's `printf` so that the output stream, its buffering
    // mode, and flush-at-exit behavior are byte-for-byte identical to the
    // original C library (which writes through stdio).
    #[link_name = "printf"]
    safe fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// Mirrors `static void print_hex(unsigned char *p, int len)`.
///
/// Prints each byte as two lowercase hex digits, then a newline.
fn print_hex(p: &[u8], len: c_int) {
    // Format strings are NUL-terminated byte literals, matching the C source.
    const FMT_BYTE: &[u8; 5] = b"%02x\0";
    const FMT_NL: &[u8; 2] = b"\n\0";

    let mut i: c_int = 0;
    while i < len {
        c_printf(
            FMT_BYTE.as_ptr() as *const c_char,
            // `unsigned char` is promoted to `int` when passed as a variadic
            // argument, so widen without sign extension.
            c_int::from(p[i as usize]),
        );
        i += 1;
    }
    c_printf(FMT_NL.as_ptr() as *const c_char);
}

/// Mirrors `void driver(int x)`.
///
/// Copies the raw object representation of `x` into a local buffer and dumps
/// it as hex. The result therefore reflects the host's integer endianness,
/// exactly as the C `memcpy` does.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [u8; core::mem::size_of::<c_int>()] = x.to_ne_bytes();
    print_hex(&raw, raw.len() as c_int);
}
