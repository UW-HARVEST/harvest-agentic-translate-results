// Rust translation of c_src/src/driver.c (public header: c_src/include/driver.h)
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

// The C code emits its output through the C standard library's stdout stream
// (`printf` / `putchar` -- gcc lowers `printf("\n")` to `putchar('\n')`).
// Calling the very same libc entry points keeps buffering, flush ordering and
// the resulting byte stream identical to the C library.
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
}

/// static void print_hex(unsigned char *p, int len)
///
/// Prints each byte as two lowercase hex digits, then a newline.
fn print_hex(p: *const c_uchar, len: c_int) {
    // `for (int i = 0; i < len; i++)` -- a non-positive `len` prints nothing.
    let mut i: c_int = 0;
    while i < len {
        // printf("%02x", p[i]) -- the unsigned char is promoted to int.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(b"%02x\0".as_ptr() as *const c_char, byte as c_int);
        }
        i += 1;
    }
    // printf("\n")
    unsafe {
        putchar(b'\n' as c_int);
    }
}

/// void driver(int x)
///
/// Copies the raw object representation of `x` into a local buffer and dumps
/// it byte-by-byte as hex (host byte order, exactly as the C code does).
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));
    let raw: [c_char; core::mem::size_of::<c_int>()] =
        unsafe { core::mem::transmute::<c_int, [c_char; core::mem::size_of::<c_int>()]>(x) };

    // print_hex((unsigned char *)raw, sizeof(raw));
    print_hex(
        raw.as_ptr() as *const c_uchar,
        core::mem::size_of_val(&raw) as c_int,
    );
}
