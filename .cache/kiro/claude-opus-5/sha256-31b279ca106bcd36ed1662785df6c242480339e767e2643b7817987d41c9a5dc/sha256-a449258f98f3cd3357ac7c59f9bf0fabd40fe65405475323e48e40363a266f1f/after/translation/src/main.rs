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
//
// The translation intentionally reproduces the original defects (including the
// out-of-bounds `strncpy` with a negative length, which faults at runtime) and
// C's stdio buffering semantics so that observable output is byte-identical.

use std::io::{ErrorKind, IsTerminal, Read, Write};

/// Emulation of C's `stdout`: line buffered when attached to a terminal,
/// fully buffered otherwise. Data that is still sitting in the buffer when the
/// process dies abnormally is lost, exactly as with glibc.
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

    /// Equivalent of `printf("%s\n", bytes)`.
    fn print_bytes_ln(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        self.buf.push(b'\n');
        if self.line_buffered {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let out = std::io::stdout();
        let mut lock = out.lock();
        let _ = lock.write_all(&self.buf);
        let _ = lock.flush();
        self.buf.clear();
    }
}

/// void printLine(const char * line)
fn print_line(out: &mut CStdout, line: Option<&[u8]>) {
    if let Some(line) = line {
        out.print_bytes_ln(line);
    }
}

/// Read a single byte from `r`, returning None on EOF or error.
fn getc(r: &mut impl Read) -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match r.read(&mut b) {
            Ok(0) => return None,
            Ok(_) => return Some(b[0]),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// Emulation of `fgets(buffer, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept).
/// Returns `None` (C's NULL) when EOF is hit before any byte is read.
fn fgets(r: &mut impl Read, size: usize) -> Option<Vec<u8>> {
    if size == 0 {
        return None;
    }
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < size - 1 {
        match getc(r) {
            Some(c) => {
                buf.push(c);
                if c == b'\n' {
                    break;
                }
            }
            None => {
                if buf.is_empty() {
                    return None;
                }
                break;
            }
        }
    }
    Some(buf)
}

/// Emulation of glibc's `atoi`: `(int) strtol(s, NULL, 10)`.
///
/// Leading whitespace is skipped, an optional sign is accepted, digits are
/// accumulated with saturation at the `long` (i64) bounds, and the result is
/// truncated to `int` (32 bits), matching the platform's wrapping conversion.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | b'\x0c' | b'\r') {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }
    let value: i64 = if overflow {
        // strtol saturates at LONG_MAX / LONG_MIN.
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };
    // (int) cast: wrapping truncation to 32 bits.
    value as i32
}

/// Reproduce the fault caused by the original `strncpy(dest, source, data)`
/// call with a negative length (converted to a huge `size_t`), which runs off
/// the end of the 100 byte destination buffer. Buffered stdout data is
/// deliberately *not* flushed, mirroring the aborted process.
fn out_of_bounds_write_fault() -> ! {
    // A wild store, as performed by the original C code.
    unsafe {
        std::ptr::write_volatile(std::ptr::null_mut::<u8>(), 0u8);
    }
    // Should never be reached; keep the process from continuing regardless.
    std::process::abort();
}

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, which makes a
/// write to a closed pipe fail with `EPIPE` instead of killing the process. A
/// C program keeps the default disposition and dies with `SIGPIPE`, so restore
/// it here to keep the observable exit status identical.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let mut out = CStdout::new();

    let mut data: i32 = -1;
    {
        // char inputBuffer[14] = "";
        let stdin = std::io::stdin();
        let mut lock = stdin.lock();
        match fgets(&mut lock, 14) {
            Some(input_buffer) => {
                /* Convert to int */
                data = atoi(&input_buffer);
            }
            None => {
                print_line(&mut out, Some(b"fgets() failed."));
            }
        }
    }
    {
        // char source[100]; char dest[100] = "";
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];
        for b in source.iter_mut().take(100 - 1) {
            *b = b'A';
        }
        source[100 - 1] = 0;

        if data < 100 {
            if data < 0 {
                // strncpy with a negative (i.e. enormous) length.
                out_of_bounds_write_fault();
            }
            let n = data as usize;
            // strncpy(dest, source, n): copy up to the source's NUL, then pad
            // the remainder of the n bytes with NUL (dest is already zeroed).
            let mut i = 0usize;
            while i < n {
                let c = source[i];
                if c == 0 {
                    break;
                }
                dest[i] = c;
                i += 1;
            }
            dest[n] = 0;
        }
        let len = dest.iter().position(|&c| c == 0).unwrap_or(dest.len());
        print_line(&mut out, Some(&dest[..len]));
    }

    out.flush();
}
