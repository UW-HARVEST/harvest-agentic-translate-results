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

//! Rust translation of c_src/src/main.c.
//!
//! The C program reads two lines with `fgets` into 100-byte buffers, strips the
//! final byte of each, and prints `strcspn(s1, s2)`.

use std::io::{self, Read, Write};

/// `strlen`: index of the first NUL byte, or the whole buffer if none exists.
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// `strcspn(s1, s2)`: length of the initial run of `s1` containing no byte that
/// appears in `s2`. Both operands are NUL-terminated C strings, so the scan of
/// `s1` stops at its NUL and the reject set excludes `s2`'s terminator.
fn c_strcspn(s1: &[u8], s2: &[u8]) -> usize {
    let reject = &s2[..c_strlen(s2)];
    let n1 = c_strlen(s1);
    for i in 0..n1 {
        if reject.contains(&s1[i]) {
            return i;
        }
    }
    n1
}

/// `fgets(buf, buf.len(), stdin)`.
///
/// Reads at most `buf.len() - 1` bytes, stopping after a newline (which is kept
/// in the buffer) or at EOF, then NUL-terminates. Returns `false` (C's NULL)
/// when EOF is hit before any byte is read; in that case `buf` is left
/// untouched, matching glibc.
fn c_fgets<R: Read>(buf: &mut [u8], reader: &mut R) -> bool {
    let max = buf.len() - 1;
    let mut n = 0usize;
    let mut byte = [0u8; 1];
    while n < max {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf[n] = byte[0];
                n += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    if n == 0 {
        return false;
    }
    buf[n] = 0;
    true
}

/// `s[strlen(s) - 1] = '\0'`.
///
/// When the string is empty the C code computes `s[-1]`, an out-of-bounds
/// write. Both buffers are zero-initialized and `fgets` with size 100 never
/// stores past index 99, so whichever byte precedes the array already holds
/// zero and the stray store has no observable effect. It is therefore skipped
/// here rather than emulated.
fn strip_last_byte(buf: &mut [u8]) {
    let len = c_strlen(buf);
    if len > 0 {
        buf[len - 1] = 0;
    }
}

fn driver(s1: &[u8], s2: &[u8], out: &mut dyn Write) {
    // printf("%zu\n", strcspn(s1, s2));
    let _ = write!(out, "{}\n", c_strcspn(s1, s2));
}

fn main() {
    let mut s1 = [0u8; 100];
    let mut s2 = [0u8; 100];

    let stdin = io::stdin();
    let mut reader = stdin.lock();

    c_fgets(&mut s1, &mut reader);
    // The C code passes sizeof(s1) for the second read; both buffers are 100 bytes.
    c_fgets(&mut s2, &mut reader);

    strip_last_byte(&mut s1);
    strip_last_byte(&mut s2);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&s1, &s2, &mut out);
    let _ = out.flush();
}
