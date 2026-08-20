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
//
// Rust translation of the original C `driver` program. The C code reads two
// lines with fgets() into 100-byte zero-initialized buffers, unconditionally
// chops off the final byte of each (which truncates real data when the line
// lacks a trailing newline), and prints strcspn(s1, s2) with "%zu\n".

use std::io::{self, BufRead, BufReader, Write};

/// C `strlen` over a fixed-size NUL-terminated byte buffer: the index of the
/// first NUL byte.
fn strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(i) => i,
        // Cannot happen for our buffers (they always contain a NUL), but keep
        // the C-like fallback of "whole buffer".
        None => buf.len(),
    }
}

/// C `strcspn(s1, s2)`: length of the initial segment of `s1` consisting of
/// bytes not present in `s2`. Both operands are NUL-terminated buffers.
fn strcspn(s1: &[u8], s2: &[u8]) -> usize {
    let reject = &s2[..strlen(s2)];
    let s1 = &s1[..strlen(s1)];
    let mut n = 0usize;
    while n < s1.len() {
        if reject.contains(&s1[n]) {
            break;
        }
        n += 1;
    }
    n
}

/// C `fgets(buf, buf.len(), stream)` semantics: copy at most `buf.len() - 1`
/// bytes, stopping after a newline (which is kept) or at EOF, then NUL
/// terminate. Returns false (the NULL return of fgets) when EOF is hit before
/// any byte is read; in that case `buf` is left untouched.
fn fgets<R: BufRead>(buf: &mut [u8], reader: &mut R) -> bool {
    let cap = buf.len() - 1; // room for the terminating NUL
    let mut n = 0usize;
    let mut scratch = [0u8; 1];

    while n < cap {
        match reader.read(&mut scratch) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf[n] = scratch[0];
                n += 1;
                if scratch[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break, // read error: behaves like end of input
        }
    }

    if n == 0 {
        return false; // buffer untouched, just like fgets returning NULL
    }
    buf[n] = 0;
    true
}

fn driver(s1: &[u8], s2: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", strcspn(s1, s2));
    let _ = out.flush();
}

/// The C code performs `s[strlen(s) - 1] = '\0'` unconditionally. When the
/// string is empty this indexes s[-1], writing a 0 byte just before the
/// buffer. On the reference build that write only ever lands on padding or on
/// the always-NUL last byte of the neighbouring buffer, so it is observably a
/// no-op; the string itself stays empty.
fn chop_last_byte(buf: &mut [u8]) {
    let len = strlen(buf);
    if len > 0 {
        buf[len - 1] = 0;
    }
}

fn main() {
    let mut s1 = [0u8; 100];
    let mut s2 = [0u8; 100];

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    // Note: the C code passes sizeof(s1) for both calls; both buffers are 100
    // bytes, so the effective limit is the same.
    let _ = fgets(&mut s1, &mut reader);
    let _ = fgets(&mut s2, &mut reader);

    chop_last_byte(&mut s1);
    chop_last_byte(&mut s2);

    driver(&s1, &s2);
}
