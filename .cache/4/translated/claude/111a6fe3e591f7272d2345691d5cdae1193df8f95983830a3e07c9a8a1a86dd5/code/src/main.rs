// Translation of c_src/src/main.c to Rust.
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

const BUF_LEN: usize = 1000;

/// Equivalent of:
///
/// ```c
/// int foo(const char *in, char c) {
///     int res = 0;
///     for (const char *s = in; s = strchr(s, c); s++) {
///         res++;
///     }
///     return res;
/// }
/// ```
///
/// `in` is a NUL-terminated C string; `strchr` walks it looking for `c`.
/// Counting therefore stops at the first NUL byte in the buffer.
fn foo(input: &[u8], c: u8) -> i32 {
    // `input` is the NUL-terminated view of the buffer (NUL excluded).
    let mut res: i32 = 0;
    let mut s: usize = 0;
    loop {
        // strchr(s, c): search from offset `s` (inclusive) for byte `c`.
        // Note strchr can also match the terminating NUL, but the driver only
        // ever passes 'A' and 'x', so the NUL case never arises here.
        match input[s..].iter().position(|&b| b == c) {
            Some(off) => {
                res = res.wrapping_add(1);
                // s++ after the successful find.
                s = s + off + 1;
                if s > input.len() {
                    // Cannot happen: a match implies s+off < len.
                    return res;
                }
            }
            None => return res,
        }
    }
}

/// Equivalent of:
///
/// ```c
/// void driver(const char *in) {
///     printf("A: %d\n", foo(in, 'A'));
///     printf("x: %d\n", foo(in, 'x'));
/// }
/// ```
fn driver(input: &[u8], out: &mut impl Write) {
    let _ = write!(out, "A: {}\n", foo(input, b'A'));
    let _ = write!(out, "x: {}\n", foo(input, b'x'));
}

fn main() {
    // char in[1000] = "";  -> the entire array is zero-initialized.
    let mut buf = [0u8; BUF_LEN];

    // fread(in, 1, sizeof(in), stdin): read up to 1000 raw bytes from stdin.
    // fread does not stop at newlines; it keeps reading until the buffer is
    // full or EOF is reached. The return value is ignored by the C code.
    let mut stdin = io::stdin();
    let mut filled = 0usize;
    while filled < BUF_LEN {
        match stdin.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    // The C code then treats `in` as a NUL-terminated string. Since the array
    // was zero-filled, the string ends at the first zero byte (which is the
    // first byte not overwritten by fread, or an embedded NUL from the input).
    let end = buf.iter().position(|&b| b == 0).unwrap_or(BUF_LEN);
    let input = &buf[..end];

    let stdout = io::stdout();
    let mut lock = stdout.lock();
    driver(input, &mut lock);
    let _ = lock.flush();
}
