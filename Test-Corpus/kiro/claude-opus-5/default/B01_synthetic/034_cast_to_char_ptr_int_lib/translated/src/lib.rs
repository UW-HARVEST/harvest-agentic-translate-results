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

use std::ffi::c_int;
use std::io::Write;

/// Mirrors the `static void print_hex(unsigned char *p, int len)` helper:
/// prints each byte as two lowercase hex digits, then a newline.
///
/// A negative `len` yields no byte output (the C `for` loop body never runs),
/// only the trailing newline; that behavior is preserved.
fn print_hex(p: &[u8], len: c_int) {
    // Build the output in one buffer so it reaches stdout as a single write,
    // matching the contiguous bytes emitted by the C printf calls.
    let mut out = Vec::with_capacity((len.max(0) as usize) * 2 + 1);
    let mut i: c_int = 0;
    while i < len {
        // `printf("%02x", p[i])` on an unsigned char: two lowercase hex digits.
        let byte = p[i as usize];
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0f));
        i += 1;
    }
    out.push(b'\n');

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    // Ignore write errors, as C's printf return value is ignored here.
    let _ = lock.write_all(&out);
    // C's stdio flushes at process exit; flush explicitly so the output is
    // emitted even when this library is driven from a non-Rust main().
    let _ = lock.flush();
}

#[inline]
fn hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'a' + (nibble - 10),
    }
}

/// void driver(int x);
///
/// Prints the object representation of `x` (native byte order) as hex.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    // (unsigned char *)&x over sizeof(x) bytes.
    let bytes = x.to_ne_bytes();
    print_hex(&bytes, size_of::<c_int>() as c_int);
}
