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

use std::io::Write;

// Raw libc entry points. Declared directly so the crate keeps zero
// dependencies; all are part of the platform C library the C program links to.
extern "C" {
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
    fn signal(signum: i32, handler: usize) -> usize;
    #[link_name = "__errno_location"]
    fn errno_location() -> *mut i32;
}

const STDIN_FILENO: i32 = 0;
const SEEK_CUR: i32 = 1;
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
const EINTR: i32 = 4;

/// Reads a single byte straight from file descriptor 0, with no buffering.
///
/// `scanf` is emulated byte by byte rather than through a buffered reader so
/// that the process consumes exactly as much of stdin as the C program does.
/// glibc reads ahead into its stdio buffer but repositions a seekable stream to
/// the logical stream position when the stream is cleaned up at exit, so the
/// externally visible effect is that only the bytes the conversion actually
/// used are consumed. A buffered Rust reader would instead swallow the whole
/// first 8 KiB block, which a later reader of the same descriptor would notice.
fn read_byte() -> Option<u8> {
    let mut byte = 0u8;
    loop {
        let n = unsafe { read(STDIN_FILENO, &mut byte, 1) };
        if n == 1 {
            return Some(byte);
        }
        if n == 0 {
            return None; // EOF
        }
        // Retry on EINTR, exactly as glibc's stdio does; treat other errors as EOF.
        if unsafe { *errno_location() } != EINTR {
            return None;
        }
    }
}

/// Pushes the most recently read byte back, mirroring the `ungetc` that `scanf`
/// performs for the character that terminates a conversion. On a pipe or other
/// non-seekable descriptor this fails harmlessly, which is also unobservable:
/// glibc cannot restore a consumed pipe byte either.
fn unread_byte() {
    unsafe {
        lseek(STDIN_FILENO, -1, SEEK_CUR);
    }
}

/// Minimal emulation of C's `scanf("%d", &x)` over stdin.
///
/// Mirrors the C library behaviour: leading whitespace (including newlines) is
/// skipped, an optional sign may follow, then decimal digits are consumed until
/// a non-digit is seen. Returns `Some(value)` on a successful conversion (the
/// `scanf` return value would be 1) and `None` on a matching failure or EOF
/// before any digit (`0` or `EOF`), in which case the caller leaves the
/// destination variable untouched.
///
/// As with glibc, a value that does not fit in a C `long` saturates at
/// `LONG_MAX`/`LONG_MIN` and the result is then truncated to a C `int`.
fn scanf_int() -> Option<i32> {
    let next = read_byte;

    // Skip whitespace, exactly the set matched by C's isspace() in the C locale.
    let mut c = loop {
        match next() {
            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(0x0b) | Some(0x0c) | Some(b'\r') => {
                continue
            }
            Some(other) => break other,
            None => return None, // EOF before any conversion.
        }
    };

    // Optional sign.
    let negative = match c {
        b'-' => {
            c = match next() {
                Some(v) => v,
                None => return None, // Matching failure: sign with no digits.
            };
            true
        }
        b'+' => {
            c = match next() {
                Some(v) => v,
                None => return None,
            };
            false
        }
        _ => false,
    };

    if !c.is_ascii_digit() {
        // Matching failure. The offending character is pushed back; note that a
        // consumed sign character is *not* restored, matching glibc's single
        // character of pushback.
        unread_byte();
        return None;
    }

    // Accumulate the magnitude, saturating the way strtol does.
    let mut saturated = false;
    let mut acc: u64 = 0;
    loop {
        let digit = u64::from(c - b'0');
        match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => acc = v,
            None => saturated = true,
        }
        // glibc's strtol clamps at LONG_MAX / LONG_MIN.
        let limit = if negative {
            i64::MIN.unsigned_abs()
        } else {
            i64::MAX as u64
        };
        if acc > limit {
            saturated = true;
        }

        match next() {
            Some(v) if v.is_ascii_digit() => c = v,
            // A non-digit terminates the conversion and is pushed back.
            Some(_) => {
                unread_byte();
                break;
            }
            None => break, // EOF also terminates the conversion.
        }
    }

    let as_long: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    // Assignment of the long result to an `int` object truncates.
    Some(as_long as i32)
}

fn driver(x: i32, out: &mut dyn Write) {
    let mut i: i32 = 0;
    let mut j: i32 = 0;
    while i < x {
        // Ignore write errors, as printf's return value is ignored in the C.
        let _ = writeln!(out, "{} {}", i, j);
        i = i.wrapping_add(1);
        j = j.wrapping_add(2);
    }
}

fn main() {
    // The Rust runtime ignores SIGPIPE, but a C program inherits the default
    // disposition. Restore it so that a closed stdout terminates this process by
    // signal (status 141 through a shell) exactly as the C program does, rather
    // than silently swallowing the write error and exiting 0.
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }

    let mut x: i32 = 0;

    if let Some(v) = scanf_int() {
        x = v;
    }

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    driver(x, &mut out);
    let _ = out.flush();
}
