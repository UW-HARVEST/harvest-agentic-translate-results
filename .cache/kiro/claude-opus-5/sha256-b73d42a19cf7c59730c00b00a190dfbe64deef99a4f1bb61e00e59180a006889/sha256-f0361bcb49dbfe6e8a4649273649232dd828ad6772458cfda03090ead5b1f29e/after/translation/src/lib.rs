// Rust translation of c_src/src/driver.c
//
// Original copyright notice from the C source is reproduced below, as the
// translation is a derivative work.
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

// The C code writes with `printf`/`putchar` from libc's stdio. We bind to the
// very same functions rather than using Rust's `std::io::stdout`, so that the
// bytes written, the destination FILE stream, and stdio's buffering/flush
// semantics (including flush-at-exit via `atexit`) are identical to the C
// library's. Mixing Rust's own stdout buffer with libc's would risk reordered
// or lost output when a host process also uses stdio.
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(c: c_int) -> c_int;
}

/// `static void print_hex(unsigned char *p, int len)`
///
/// Not part of the public ABI (it is `static` in C), so it is a private Rust
/// function here and is deliberately not exported.
///
/// # Safety
/// `p` must point to at least `len` readable bytes when `len > 0`.
unsafe fn print_hex(p: *const c_uchar, len: c_int) {
    // `for (int i = 0; i < len; i++)`: a non-positive `len` iterates zero times.
    let mut i: c_int = 0;
    while i < len {
        // `printf("%02x", p[i])`: the `unsigned char` argument is promoted to
        // `int` by the default argument promotions, so pass a `c_int` here.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(c"%02x".as_ptr(), byte as c_int);
        }
        i += 1;
    }
    // `printf("\n")` in the C source; emitting the single byte directly is
    // byte-for-byte equivalent (and is what the C compiler itself lowers this
    // call to).
    unsafe {
        putchar(b'\n' as c_int);
    }
}

/// `void driver(int x)` from include/driver.h
///
/// Reinterprets the object representation of `x` as `sizeof(int)` bytes and
/// prints them in order as lowercase hex, followed by a newline. The byte order
/// is therefore the target's native endianness, matching the C `memcpy`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // `char raw[sizeof(x)]; memcpy(raw, &x, sizeof(x));`
    let raw: [u8; core::mem::size_of::<c_int>()] = x.to_ne_bytes();
    // `print_hex((unsigned char *)raw, sizeof(raw))`
    unsafe {
        print_hex(raw.as_ptr() as *const c_uchar, raw.len() as c_int);
    }
}
