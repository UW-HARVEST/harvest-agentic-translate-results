// Rust translation of c_src/src/main.c
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
//
// The translation intentionally reproduces the original (buggy) behavior of the
// C program, including the negative-length `strncpy` call that the C code
// performs when the parsed value is negative.

use std::io::{IsTerminal, Read, Write};

/// Mimics C's `stdout` buffering discipline: line buffered when attached to a
/// terminal, fully buffered otherwise (data is only written at flush time).
/// This matters because the program can die from a memory fault before any
/// flush happens, in which case a fully buffered stream loses its contents.
struct CStdout {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    fn new() -> Self {
        CStdout {
            buf: Vec::new(),
            line_buffered: std::io::stdout().is_terminal(),
        }
    }

    /// `printf("%s\n", line)` on a NUL-terminated byte string.
    fn printf_line(&mut self, line: &[u8]) {
        self.buf.extend_from_slice(line);
        self.buf.push(b'\n');
        if self.line_buffered {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(&self.buf);
        let _ = handle.flush();
        self.buf.clear();
    }
}

/// void printLine (const char * line)
fn print_line(out: &mut CStdout, line: Option<&[u8]>) {
    if let Some(line) = line {
        out.printf_line(line);
    }
}

/// Length of a NUL-terminated string held in a byte buffer (`strlen`).
fn c_strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(pos) => pos,
        None => buf.len(),
    }
}

/// Borrow the NUL-terminated string stored at the start of `buf`.
fn c_str(buf: &[u8]) -> &[u8] {
    &buf[..c_strlen(buf)]
}

/// `fgets(buf, size, stdin)` for a buffer of `buf.len()` bytes.
///
/// Reads at most `buf.len() - 1` bytes, stopping right after a newline, and
/// NUL-terminates.  Returns `false` (the C `NULL` return) when end-of-file or
/// an error is hit before any character is read.
fn fgets(buf: &mut [u8]) -> bool {
    let limit = buf.len() - 1;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut count = 0usize;
    let mut byte = [0u8; 1];

    while count < limit {
        match handle.read(&mut byte) {
            Ok(0) => break,                 // EOF
            Ok(_) => {
                buf[count] = byte[0];
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,                // read error
        }
    }

    if count == 0 {
        return false;
    }
    buf[count] = 0;
    true
}

/// `atoi(nptr)`, i.e. glibc's `(int) strtol(nptr, NULL, 10)`:
/// skip leading whitespace, optional sign, decimal digits, saturate at the
/// `long` limits, then truncate to `int`.
fn atoi(s: &[u8]) -> i32 {
    let s = c_str(s);
    let mut i = 0usize;

    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut value: i64 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !saturated {
            match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => value = v,
                None => saturated = true,
            }
        }
        i += 1;
    }

    let result: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        value.wrapping_neg()
    } else {
        value
    };

    result as i32
}

/// `strncpy(dest, src, n)` faithful to glibc: copy up to `n` bytes of the
/// NUL-terminated `src`, then pad the remainder of the `n` bytes with NUL.
///
/// The C program calls this with a negative `int` length that converts to a
/// huge `size_t`, so the padding step walks far past the end of the
/// destination buffer and faults.  That fault is part of the observable
/// behavior being reproduced here, so the write is performed for real.
fn strncpy(dest: &mut [u8; 100], src: &[u8; 100], n: usize) {
    let src_len = c_strlen(src);
    let to_copy = if src_len < n { src_len } else { n };

    if to_copy <= dest.len() {
        dest[..to_copy].copy_from_slice(&src[..to_copy]);
    } else {
        // Not reachable for this program (src_len is 99), kept for fidelity.
        unsafe {
            std::ptr::copy_nonoverlapping(src.as_ptr(), dest.as_mut_ptr(), to_copy);
        }
    }

    if n > to_copy {
        let pad = n - to_copy;
        if to_copy + pad <= dest.len() {
            for b in dest[to_copy..to_copy + pad].iter_mut() {
                *b = 0;
            }
        } else {
            // Reproduce the out-of-bounds fill performed by the C code.
            unsafe {
                std::ptr::write_bytes(dest.as_mut_ptr().add(to_copy), 0u8, pad);
            }
        }
    }
}

fn main() {
    let out = &mut CStdout::new();

    let mut data: i32;
    data = -1;
    {
        let mut input_buffer = [0u8; 14];
        if fgets(&mut input_buffer) {
            /* Convert to int */
            data = atoi(&input_buffer);
        } else {
            print_line(out, Some(b"fgets() failed."));
        }
    }
    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for b in source[..100 - 1].iter_mut() {
            *b = b'A';
        }
        source[100 - 1] = 0;
        if data < 100 {
            strncpy(&mut dest, &source, data as usize);
            let idx = data as usize;
            if idx < dest.len() {
                dest[idx] = 0;
            } else {
                unsafe {
                    std::ptr::write(dest.as_mut_ptr().add(idx), 0u8);
                }
            }
        }
        let line = c_str(&dest).to_vec();
        print_line(out, Some(&line));
    }

    out.flush();
    std::process::exit(0);
}
