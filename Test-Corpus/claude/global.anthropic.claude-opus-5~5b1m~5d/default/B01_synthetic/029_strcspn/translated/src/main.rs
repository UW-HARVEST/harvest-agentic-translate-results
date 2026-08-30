// Rust translation of c_src/src/main.c
//
// Original copyright notice from the C source:
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

use std::io::{self, Read, Write};

/// Emulates C's `strlen` over a fixed-size byte buffer holding a NUL-terminated
/// string: the number of bytes before the first NUL byte.
fn c_strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(i) => i,
        // The C buffers are always NUL-terminated in this program, but be
        // defensive rather than panicking.
        None => buf.len(),
    }
}

/// Emulates C's `strcspn(s1, s2)`: the length of the maximum initial segment of
/// `s1` made up of bytes that do not appear in `s2`. The terminating NUL bytes
/// are not part of the compared sets.
fn c_strcspn(s1: &[u8], s2: &[u8]) -> usize {
    let n1 = c_strlen(s1);
    let n2 = c_strlen(s2);
    let set = &s2[..n2];
    for i in 0..n1 {
        if set.contains(&s1[i]) {
            return i;
        }
    }
    n1
}

/// Emulates C's `fgets(buf, buf.len(), stdin)`.
///
/// Reads bytes (including any embedded NUL bytes) until a newline is stored, EOF
/// is reached, or `buf.len() - 1` bytes have been stored; a terminating NUL is
/// then written. Returns `false` (C's NULL) when EOF is hit before any byte is
/// read, in which case the buffer is left untouched -- exactly as C does.
fn c_fgets<R: Read>(buf: &mut [u8], input: &mut R) -> bool {
    if buf.is_empty() {
        return false;
    }
    let limit = buf.len() - 1;
    let mut count = 0usize;
    let mut byte = [0u8; 1];
    while count < limit {
        match input.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buf[count] = byte[0];
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break, // read error: behaves like EOF here
        }
    }
    if count == 0 {
        return false; // NULL: buffer is not modified
    }
    buf[count] = 0;
    true
}

fn driver(s1: &[u8], s2: &[u8]) {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    // printf("%zu\n", strcspn(s1, s2));
    let _ = write!(out, "{}\n", c_strcspn(s1, s2));
    let _ = out.flush();
}

fn main() {
    // char s1[100] = "", s2[100] = "";  -> fully zero-initialized
    let mut s1 = [0u8; 100];
    let mut s2 = [0u8; 100];

    let stdin = io::stdin();
    let mut input = io::BufReader::new(stdin.lock());

    c_fgets(&mut s1, &mut input);
    // NOTE: the C code passes sizeof(s1) for the second read as well; both
    // buffers are 100 bytes, so the behavior is identical.
    c_fgets(&mut s2, &mut input);

    // s1[strlen(s1)-1] = '\0';
    // s2[strlen(s2)-1] = '\0';
    //
    // When the string is empty, strlen() - 1 wraps to SIZE_MAX and the write
    // lands one byte *before* the array (undefined behavior). In practice that
    // byte is either padding or the already-NUL final byte of the neighbouring
    // 100-byte buffer, so the store is unobservable; we reproduce it as a no-op.
    let n1 = c_strlen(&s1);
    if n1 > 0 {
        s1[n1 - 1] = 0;
    }
    let n2 = c_strlen(&s2);
    if n2 > 0 {
        s2[n2 - 1] = 0;
    }

    driver(&s1, &s2);
}
