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

use std::io::{Read, IsTerminal, Write};

extern "C" {
    /// libc raise(), used to reproduce the fatal signal the C program dies with.
    fn raise(sig: i32) -> i32;
    /// libc signal(), used to restore the default SIGSEGV disposition (the Rust
    /// runtime installs its own handler for stack-overflow reporting).
    fn signal(sig: i32, handler: usize) -> usize;
}

const SIGSEGV: i32 = 11;
const SIG_DFL: usize = 0;

/// Emulation of C's `stdout`: fully buffered when stdout is not a terminal,
/// line buffered when it is.  This matters because the original program can die
/// from a fatal signal with data still sitting in the FILE buffer (that data is
/// then never written), so the buffering mode is observable.
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

    /// `printf`-style write of raw bytes.
    fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
        if self.line_buffered && self.buf.contains(&b'\n') {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if !self.buf.is_empty() {
            let out = std::io::stdout();
            let mut lock = out.lock();
            let _ = lock.write_all(&self.buf);
            let _ = lock.flush();
            self.buf.clear();
        }
    }
}

/// void printLine (const char * line)
/// `line` is modelled as an optional NUL-terminated byte string; `printf("%s\n", line)`
/// stops at the first NUL byte.
fn print_line(out: &mut CStdout, line: Option<&[u8]>) {
    if let Some(line) = line {
        let end = line.iter().position(|&c| c == 0).unwrap_or(line.len());
        let mut bytes = Vec::with_capacity(end + 1);
        bytes.extend_from_slice(&line[..end]);
        bytes.push(b'\n');
        out.write_bytes(&bytes);
    }
}

/// `fgets(buf, size, stdin)`: reads at most `size - 1` bytes, stopping after a
/// newline (which is kept) or at EOF.  NUL-terminates what it read.  Returns
/// false (NULL in C) if EOF was hit before any byte was read.
fn fgets(buf: &mut [u8], size: usize) -> bool {
    if size == 0 {
        return false;
    }
    let limit = size - 1;
    let mut n = 0usize;
    let stdin = std::io::stdin();
    let mut lock = stdin.lock();
    let mut byte = [0u8; 1];
    while n < limit {
        match lock.read(&mut byte) {
            Ok(0) => break,           // EOF
            Ok(_) => {
                buf[n] = byte[0];
                n += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(_) => break,          // read error
        }
    }
    if n == 0 {
        return false;
    }
    buf[n] = 0;
    true
}

/// `atoi()` as implemented by glibc: `(int) strtol(nptr, NULL, 10)`.
/// Parses the NUL-terminated contents of `buf`.
fn atoi(buf: &[u8]) -> i32 {
    let s = &buf[..buf.iter().position(|&c| c == 0).unwrap_or(buf.len())];
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
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = i64::from(s[i] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => acc = v,
                None => saturated = true, // strtol clamps to LONG_MAX / LONG_MIN
            }
        }
        i += 1;
    }
    let value: i64 = if saturated {
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
    value as i32 // truncating cast, as (int) of a long
}

/// Reproduces the fatal SIGSEGV of the original program: `strncpy(dest, source, data)`
/// with a negative `data` passes a huge `size_t`, so strncpy runs off the end of
/// `dest` and the process dies.  Nothing buffered in stdout is flushed.
fn c_crash() -> ! {
    unsafe {
        signal(SIGSEGV, SIG_DFL);
        raise(SIGSEGV);
    }
    std::process::abort();
}

fn main() {
    let out = &mut CStdout::new();

    let mut data: i32 = -1;
    {
        let mut input_buffer = [0u8; 14]; // char inputBuffer[14] = "";
        if fgets(&mut input_buffer, 14) {
            /* Convert to int */
            data = atoi(&input_buffer);
        } else {
            print_line(out, Some(b"fgets() failed."));
        }
    }
    {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100]; // char dest[100] = "";
        for b in source[..100 - 1].iter_mut() {
            *b = b'A';
        }
        source[100 - 1] = 0;
        if data < 100 {
            if data < 0 {
                // strncpy() with a negative length -> enormous size_t -> crash
                c_crash();
            }
            let n = data as usize;
            // strncpy(dest, source, n): copy up to n bytes, stopping after a NUL,
            // then zero-pad the remainder of the n bytes.
            let mut i = 0usize;
            while i < n && source[i] != 0 {
                dest[i] = source[i];
                i += 1;
            }
            while i < n {
                dest[i] = 0;
                i += 1;
            }
            dest[n] = 0;
        }
        print_line(out, Some(&dest));
    }

    out.flush();
    std::process::exit(0);
}
