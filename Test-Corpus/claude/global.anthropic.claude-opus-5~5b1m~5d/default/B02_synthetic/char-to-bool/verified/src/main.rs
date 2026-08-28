/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Direct translation of `c_src/src/main.c`.

use std::io::{self, BufRead, Write};

use driver::process_decisions;

const MAX_INPUT_SIZE: usize = 1024;

/// Emulates C's `fgets(buf, size, stdin)`.
///
/// Reads bytes until (and including) a newline, until `size - 1` bytes have
/// been stored, or until end-of-file.  Returns `None` for the `NULL` return of
/// `fgets`, otherwise the bytes stored in the buffer (the terminating NUL is
/// implicit).
///
/// C7.21.7.2: `fgets` returns a null pointer if end-of-file is encountered
/// before *any* character has been read, and also if a read error occurs -
/// in the latter case even when characters were already stored.
fn fgets<R: BufRead>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    let max = size - 1;
    let mut buf: Vec<u8> = Vec::new();
    let mut read_error = false;

    while buf.len() < max {
        let mut byte = [0u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => break, /* EOF */
            Ok(_) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => {
                read_error = true;
                break;
            }
        }
    }

    if read_error || buf.is_empty() {
        None
    } else {
        Some(buf)
    }
}

/// Length of the NUL-terminated C string held in `buf`, i.e. `strlen`.
fn strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&c| c == 0).unwrap_or(buf.len())
}

/// Emulates glibc's `atoi`, which is `(int) strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is accepted, decimal digits
/// are consumed, out-of-range values saturate to `LONG_MIN`/`LONG_MAX` and the
/// result is then truncated to `int`.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;

    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
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
        let digit = i64::from(s[i] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if overflow {
        /* strtol saturates, then the cast to int truncates. */
        return if negative {
            i64::MIN as i32
        } else {
            i64::MAX as i32
        };
    }

    let value = if negative { -acc } else { acc };
    value as i32
}

/// Restore the default disposition of `SIGPIPE`.
///
/// Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a write to
/// a pipe with no reader returns `EPIPE` and `print!` panics (aborting with
/// `SIGABRT`).  A C program runs with the default disposition and is killed by
/// `SIGPIPE` instead, so restoring it keeps the two exit statuses identical.
#[cfg(unix)]
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

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

fn main() {
    restore_default_sigpipe();
    let code = run();
    /* Flush explicitly: C's exit() flushes stdio streams. */
    let _ = io::stdout().flush();
    std::process::exit(code);
}

fn run() -> i32 {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let operation: i32;
    let param: i32;

    /* Read operation number */
    match fgets(&mut stdin, MAX_INPUT_SIZE) {
        None => {
            /* C ignores fprintf's return value. */
            let _ = write!(io::stderr(), "Error reading operation\n");
            return 1;
        }
        Some(input_buffer) => {
            operation = atoi(&input_buffer[..strlen(&input_buffer)]);
        }
    }

    /* Read parameter */
    match fgets(&mut stdin, MAX_INPUT_SIZE) {
        None => {
            let _ = write!(io::stderr(), "Error reading parameter\n");
            return 1;
        }
        Some(input_buffer) => {
            param = atoi(&input_buffer[..strlen(&input_buffer)]);
        }
    }

    /* Read decision string */
    let mut input_buffer = match fgets(&mut stdin, MAX_INPUT_SIZE) {
        None => {
            let _ = write!(io::stderr(), "Error reading decision string\n");
            return 1;
        }
        Some(buf) => buf,
    };

    /* Remove trailing newline if present */
    let mut len = strlen(&input_buffer);
    if len > 0 && input_buffer[len - 1] == b'\n' {
        input_buffer[len - 1] = 0;
        len -= 1;
    }

    /* Call the library function */
    let result = process_decisions(Some(&input_buffer[..len]), len, operation, param);

    /* Print result to stdout */
    /* C ignores printf's return value; a failed write is not an error. */
    let _ = write!(io::stdout(), "{}\n", result);

    0
}
