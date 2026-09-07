// Rust translation of c_src/ (MIT Lincoln Laboratory `driver` library).
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

use std::ffi::{c_char, c_int};

// The C code emits its output with `printf`, i.e. through libc's `stdout`
// stream. We bind to libc `printf` directly rather than using Rust's
// `std::io::stdout`, so that buffering, flush-at-exit behaviour and any
// interleaving with output produced by other C code in the same process are
// bit-for-bit identical to the original library.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// Translation of the `static void print_hex(unsigned char *p, int len)` helper
/// in `c_src/src/driver.c`.
///
/// The C original has internal linkage, so this stays private to the crate and
/// is deliberately *not* exported from the shared object.
fn print_hex(p: *const u8, len: c_int) {
    // `for (int i = 0; i < len; i++) printf("%02x", p[i]);`
    //
    // `len` is a signed int in C; a non-positive `len` simply produces no
    // iterations, which `0..len` reproduces.
    let mut i: c_int = 0;
    while i < len {
        // C promotes the `unsigned char` lvalue `p[i]` to `int` for the
        // variadic call, so pass a `c_int` here.
        let byte = unsafe { *p.offset(i as isize) };
        unsafe {
            printf(c"%02x".as_ptr(), byte as c_int);
        }
        i += 1;
    }

    // `printf("\n");`
    unsafe {
        printf(c"\n".as_ptr());
    }
}

/// Translation of `void driver(float x)` from `c_src/src/driver.c`.
///
/// Copies the object representation of `x` into a local buffer and prints it as
/// lowercase, zero-padded, two-digit hex bytes followed by a newline. The bytes
/// are emitted in the target's native order, exactly as the C `memcpy` of the
/// `float` does (little-endian on x86-64 / AArch64).
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    // char raw[sizeof(x)];
    // memcpy(raw, &x, sizeof(x));
    //
    // `f32::to_ne_bytes` is exactly a reinterpretation of the float's object
    // representation, matching the C `memcpy`: no NaN canonicalisation and no
    // byte reordering takes place.
    let raw: [u8; std::mem::size_of::<f32>()] = x.to_ne_bytes();

    // print_hex((unsigned char *)raw, sizeof(raw));
    print_hex(raw.as_ptr(), raw.len() as c_int);
}
