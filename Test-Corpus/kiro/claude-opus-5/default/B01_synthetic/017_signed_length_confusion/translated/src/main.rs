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

//! Rust translation of `c_src/src/main.c`.
//!
//! The original program is a CWE test case: it reads a line with `fgets`,
//! converts it with `atoi`, and then calls `strncpy(dest, source, data)` with a
//! *signed* length that is implicitly converted to `size_t`. A negative `data`
//! therefore becomes an enormous unsigned count and the copy runs off the end of
//! the destination stack buffer.
//!
//! This translation reproduces the observable behavior of the compiled C exactly,
//! bugs included:
//!   * `atoi` truncation semantics (`(int) strtol(...)`).
//!   * `fgets` line semantics with a 14 byte buffer (13 payload bytes max).
//!   * glibc stdio buffering: stdout is line buffered on a terminal and fully
//!     buffered otherwise, so pending output is *lost* when the process dies.
//!   * the negative-length `strncpy` crashing with SIGSEGV (exit status 139).

use std::io::{Read, Write};

/// Emulates C's `stdout` FILE stream closely enough to match observable output.
///
/// glibc picks line buffering when stdout refers to a terminal and full
/// buffering otherwise. That distinction matters here: on the `fgets() failed.`
/// path the program crashes immediately afterwards, so with full buffering the
/// message never reaches the file descriptor.
struct CStdout {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl CStdout {
    /// glibc's default fully-buffered block size (`BUFSIZ` / `st_blksize`).
    const BUFSIZ: usize = 4096;

    fn new() -> Self {
        CStdout {
            buf: Vec::with_capacity(Self::BUFSIZ),
            line_buffered: std::io::IsTerminal::is_terminal(&std::io::stdout()),
        }
    }

    /// `printf("%s\n", line)` where `line` is a NUL-terminated byte buffer.
    fn print_line(&mut self, line: &[u8]) {
        let end = line.iter().position(|&b| b == 0).unwrap_or(line.len());
        self.buf.extend_from_slice(&line[..end]);
        self.buf.push(b'\n');

        if self.line_buffered || self.buf.len() >= Self::BUFSIZ {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let out = std::io::stdout();
        let mut lock = out.lock();
        // Mirror C: a write failure on stdout is not checked by the original.
        let _ = lock.write_all(&self.buf);
        let _ = lock.flush();
        self.buf.clear();
    }
}

/// Reproduces the hardware trap taken by the original program when the
/// out-of-bounds `strncpy` walks off the stack.
///
/// Note that stdout is deliberately *not* flushed here, matching the C program
/// where buffered data is discarded when the process is killed by SIGSEGV.
fn raise_segfault() -> ! {
    // SAFETY: none. This is an intentional null dereference used to reproduce
    // the SIGSEGV (exit status 139) of the original undefined behavior.
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 0u8);
    }
    // Unreachable in practice; keeps the `!` return type honest.
    std::process::abort();
}

/// `fgets(buffer, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept) or
/// at end of file, and NUL-terminates. Returns `false` (C's `NULL`) when end of
/// file is reached before any byte is read. Unlike `scanf`, it never reads past
/// the first newline.
fn fgets(buffer: &mut [u8], size: usize) -> bool {
    if size == 0 {
        return false;
    }

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let limit = size - 1;
    let mut count = 0usize;
    let mut byte = [0u8; 1];

    while count < limit {
        match handle.read(&mut byte) {
            Ok(0) => break, // EOF
            Ok(_) => {
                buffer[count] = byte[0];
                count += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    if count == 0 {
        return false;
    }

    buffer[count] = 0;
    true
}

/// `atoi(s)`, which glibc implements as `(int) strtol(s, NULL, 10)`.
///
/// Leading whitespace is skipped, an optional sign is accepted, and conversion
/// stops at the first non-digit. Out-of-range values saturate at `long` bounds
/// and are then truncated to `int`.
fn atoi(s: &[u8]) -> i32 {
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    let s = &s[..end];
    let mut i = 0usize;

    // isspace() in the C locale.
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }

    let negative = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

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

    // strtol clamps to LONG_MAX / LONG_MIN, then atoi's cast truncates to int.
    let as_long: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -value
    } else {
        value
    };

    as_long as i32
}

/// `strncpy(dest, src, n)`.
///
/// Copies bytes from `src` until its NUL terminator, then pads `dest` with NUL
/// bytes until `n` bytes total have been written. `dest` is *not* terminated
/// when `n` bytes were consumed by the copy.
///
/// The original passes a negative `int` here, so `n` becomes astronomically
/// large and the write leaves the 100 byte stack buffer. That is reproduced as a
/// SIGSEGV rather than being silently clamped.
fn strncpy(dest: &mut [u8], src: &[u8], n: usize) {
    if n > dest.len() {
        // The C program writes out of bounds at this point; the process dies
        // before any further output is produced.
        raise_segfault();
    }

    let mut i = 0usize;
    while i < n {
        let byte = if i < src.len() { src[i] } else { 0 };
        if byte == 0 {
            break;
        }
        dest[i] = byte;
        i += 1;
    }
    while i < n {
        dest[i] = 0;
        i += 1;
    }
}

fn print_line(out: &mut CStdout, line: Option<&[u8]>) {
    if let Some(line) = line {
        out.print_line(line);
    }
}

fn main() {
    let mut out = CStdout::new();

    let mut data: i32 = -1;
    {
        let mut input_buffer = [0u8; 14];
        if fgets(&mut input_buffer, 14) {
            /* Convert to int */
            data = atoi(&input_buffer);
        } else {
            print_line(&mut out, Some(b"fgets() failed.\0"));
        }
    }
    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for slot in source.iter_mut().take(100 - 1) {
            *slot = b'A';
        }
        source[100 - 1] = 0;
        if data < 100 {
            // `data` is implicitly converted to size_t, exactly as in the C.
            strncpy(&mut dest, &source, data as usize);
            // Likewise, dest[data] with a negative index is an out-of-bounds
            // write; it is unreachable because strncpy already trapped.
            dest[data as usize] = 0;
        }
        print_line(&mut out, Some(&dest));
    }

    out.flush();
}
