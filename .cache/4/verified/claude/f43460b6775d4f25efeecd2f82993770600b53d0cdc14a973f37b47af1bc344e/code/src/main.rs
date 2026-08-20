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

//! Faithful Rust translation of `c_src/src/main.c`.

use driver::decisions;

use std::io::{self, BufReader, Read, Write};
use std::process::ExitCode;

const MAX_INPUT_SIZE: usize = 1024;

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, which a C
/// program does not: a C `main` inherits the *default* disposition, so writing to
/// a pipe with no reader terminates the process with signal 13 instead of
/// returning `EPIPE`.  Restoring `SIG_DFL` here keeps the exit status identical
/// to `c_src/src/main.c` when stdout or stderr is a broken pipe.
///
/// `signal` is declared directly rather than pulling in a `libc` dependency.
#[cfg(unix)]
fn restore_default_sigpipe() {
    /// `SIGPIPE` is 13 on Linux and every other mainstream Unix.
    const SIGPIPE: core::ffi::c_int = 13;
    /// `SIG_DFL` is the null handler.
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: core::ffi::c_int, handler: usize) -> usize;
    }

    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// `fprintf(stderr, "...")` — C ignores write errors here, so this must not
/// panic the way `eprint!` does.
fn write_stderr(msg: &str) {
    let mut err = io::stderr();
    let _ = err.write_all(msg.as_bytes());
    let _ = err.flush();
}

/// Emulates `fgets(buf, size, stdin)`.
///
/// Reads at most `size - 1` bytes, stopping after a newline (which is kept).
/// Returns `None` when EOF (or an error) is hit before any byte was read,
/// mirroring fgets returning NULL.
fn c_fgets<R: Read>(reader: &mut R, size: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();
    let capacity = size - 1;
    let mut byte = [0u8; 1];

    while out.len() < capacity {
        match reader.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                out.push(byte[0]);
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    if out.is_empty() {
        /* Nothing read: EOF/error -> NULL */
        None
    } else {
        Some(out)
    }
}

/// Bytes of the NUL-terminated C string that `fgets` left in the buffer, i.e.
/// everything up to (but not including) the first embedded NUL byte.
fn c_string(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|&b| b == 0) {
        Some(nul) => &bytes[..nul],
        None => bytes,
    }
}

/// Emulates glibc's `atoi`, which is `(int) strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is honoured, digits are
/// consumed until the first non-digit, out-of-range values saturate at
/// LONG_MIN/LONG_MAX and the result is truncated to `int`.
fn c_atoi(s: &[u8]) -> i32 {
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

    let value: i64 = if overflow {
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

    value as i32
}

fn main() -> ExitCode {
    /* Match the signal disposition a C `main` starts with (see above). */
    restore_default_sigpipe();

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    /* Read operation number */
    let buffer = match c_fgets(&mut reader, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            write_stderr("Error reading operation\n");
            return ExitCode::from(1);
        }
    };
    let operation = c_atoi(c_string(&buffer));

    /* Read parameter */
    let buffer = match c_fgets(&mut reader, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            write_stderr("Error reading parameter\n");
            return ExitCode::from(1);
        }
    };
    let param = c_atoi(c_string(&buffer));

    /* Read decision string */
    let mut buffer = match c_fgets(&mut reader, MAX_INPUT_SIZE) {
        Some(b) => b,
        None => {
            write_stderr("Error reading decision string\n");
            return ExitCode::from(1);
        }
    };

    /* Remove trailing newline if present */
    let mut len = c_string(&buffer).len(); /* strlen(input_buffer) */
    if len > 0 && buffer[len - 1] == b'\n' {
        buffer[len - 1] = 0; /* input_buffer[len - 1] = '\0' */
        len -= 1;
    }

    /* Call the library function.  Operation 3 rewrites the buffer in place,
     * just as the C code does; `main` never looks at it again afterwards. */
    let result = decisions::process_decisions(&mut buffer[..len], len, operation, param);

    /* Print result to stdout */
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", result);
    let _ = out.flush();

    ExitCode::SUCCESS
}
