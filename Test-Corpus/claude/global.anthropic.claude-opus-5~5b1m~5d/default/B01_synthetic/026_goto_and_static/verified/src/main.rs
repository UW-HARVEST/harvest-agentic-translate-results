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

// The C source uses printf with trailing newlines; print! + "\n" keeps the
// format strings byte-identical to the originals.
#![allow(clippy::print_with_newline)]

use std::io::{Read, Write};

// The C program relies on two pieces of libc behaviour that Rust's standard
// library deliberately changes:
//
//  1. SIGPIPE. A C program keeps the default disposition, so writing to a
//     closed pipe kills it with signal 13. The Rust runtime sets SIGPIPE to
//     SIG_IGN before main, turning the same situation into an `EPIPE` error
//     which `print!` escalates into a panic (exit 134 + panic message on
//     stderr). Restoring SIG_DFL makes the exit status match.
//  2. Write errors. `printf`/`fclose` failures are ignored by this C program,
//     which always `return 0`s. `print!` panics instead. `emit`/`flush_out`
//     below write through a plain buffer and discard errors.
extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn isatty(fd: i32) -> i32;
}

const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;

/// Emulates C's stdout: fully buffered when stdout is not a terminal, line
/// buffered when it is. All write errors are discarded, as the C code never
/// inspects the return value of `printf`.
struct Out {
    buf: Vec<u8>,
    line_buffered: bool,
}

impl Out {
    fn new() -> Self {
        Out {
            buf: Vec::new(),
            line_buffered: unsafe { isatty(1) } == 1,
        }
    }

    fn emit(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
        if self.line_buffered && self.buf.contains(&b'\n') {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        // Errors (EPIPE, ENOSPC, EBADF, ...) are intentionally ignored to
        // mirror the C program, which never checks printf's result.
        let _ = std::io::stdout().write_all(&self.buf);
        let _ = std::io::stdout().flush();
        self.buf.clear();
    }
}

/// A buffered byte reader over stdin with one byte of push-back, used to
/// emulate C's `scanf` character consumption semantics (which reads across
/// newlines, treating all whitespace the same).
struct Scanner {
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
    src: std::io::Stdin,
}

impl Scanner {
    fn new() -> Self {
        Scanner {
            buf: Vec::new(),
            pos: 0,
            eof: false,
            src: std::io::stdin(),
        }
    }

    /// Returns the next byte without consuming it, or None at EOF.
    fn peek(&mut self) -> Option<u8> {
        if self.pos < self.buf.len() {
            return Some(self.buf[self.pos]);
        }
        if self.eof {
            return None;
        }
        let mut chunk = [0u8; 4096];
        loop {
            match self.src.read(&mut chunk) {
                Ok(0) => {
                    self.eof = true;
                    return None;
                }
                Ok(n) => {
                    self.buf.clear();
                    self.buf.extend_from_slice(&chunk[..n]);
                    self.pos = 0;
                    return Some(self.buf[0]);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.eof = true;
                    return None;
                }
            }
        }
    }

    /// Consumes the byte returned by the most recent `peek`.
    fn bump(&mut self) {
        if self.pos < self.buf.len() {
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            // C's isspace() for the default "C" locale.
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
                self.bump();
            } else {
                break;
            }
        }
    }

    /// Emulates a single `%d` conversion. Returns Some(value) on a successful
    /// conversion, or None on input failure (EOF) or matching failure, in which
    /// case the caller must leave its destination unmodified, exactly as scanf
    /// does.
    fn scan_int(&mut self) -> Option<i32> {
        self.skip_whitespace();

        let mut negative = false;
        match self.peek() {
            None => return None, // input failure
            Some(b'-') => {
                negative = true;
                self.bump();
            }
            Some(b'+') => {
                self.bump();
            }
            Some(_) => {}
        }

        let mut saw_digit = false;
        // glibc accumulates into a long and saturates on overflow (as strtol
        // does), then truncates the result to int.
        let mut acc: i64 = 0;
        let mut overflow = false;
        while let Some(c) = self.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            saw_digit = true;
            self.bump();
            let d = (c - b'0') as i64;
            if !overflow {
                match acc.checked_mul(10).and_then(|v| v.checked_add(d)) {
                    Some(v) => acc = v,
                    None => overflow = true,
                }
            }
        }

        if !saw_digit {
            return None; // matching failure
        }

        if overflow {
            let saturated: i64 = if negative { i64::MIN } else { i64::MAX };
            return Some(saturated as i32);
        }

        let value = if negative { acc.wrapping_neg() } else { acc };
        Some(value as i32)
    }
}

// `static int y = 123;` — file-scope mutable global in the C original.
thread_local! {
    static Y: std::cell::Cell<i32> = const { std::cell::Cell::new(123) };
}

fn get_y() -> i32 {
    Y.with(|y| y.get())
}

fn set_y(v: i32) {
    Y.with(|y| y.set(v));
}

fn multi_stage(out: &mut Out, x: i32, z: i32) -> i32 {
    let mut result = 0;

    // The C function uses `goto fail`; the shared failure epilogue is modelled
    // with a labeled block that the error paths break out of.
    'fail: {
        if x != 1 {
            out.emit("Error: x != 1\n");
            result = 1;
            break 'fail;
        }

        if get_y() != 2 {
            out.emit("Error: x == 1 but y != 2\n");
            result = 2;
            break 'fail;
        }

        if z != 3 {
            out.emit("Error: x == 1 and y == 2, but z != 3\n");
            result = 3;
            break 'fail;
        }

        out.emit("Ok!\n");
        return result;
    }

    // fail:
    out.emit("Operation failed\n");
    result
}

fn main() {
    // Match the C runtime's signal disposition (see the note above).
    unsafe { signal(SIGPIPE, SIG_DFL) };

    let mut out = Out::new();
    let mut x: i32 = 0;
    let mut z: i32 = 0;

    // scanf("%d %d %d", &x, &y, &z);
    // Conversions stop at the first failure, leaving later variables untouched.
    let mut scanner = Scanner::new();
    if let Some(v) = scanner.scan_int() {
        x = v;
        if let Some(v) = scanner.scan_int() {
            set_y(v);
            if let Some(v) = scanner.scan_int() {
                z = v;
            }
        }
    }

    let result = multi_stage(&mut out, x, z);
    out.emit(&format!("Result: {}\n", result));

    // exit(0) flushes stdio streams; failures are ignored, C `return 0`s.
    out.flush();
}
