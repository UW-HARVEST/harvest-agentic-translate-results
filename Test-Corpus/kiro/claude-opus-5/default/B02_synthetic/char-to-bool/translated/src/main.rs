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

//! Rust translation of `c_src/src/main.c`.

use std::io::{self, Read, Write};
use std::process::ExitCode;

use driver::process_decisions;

const MAX_INPUT_SIZE: usize = 1024;

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    /* The C reuses a single stack buffer for all three reads. Bytes left
     * over from a previous read stay in place; they are simply never
     * looked at because `strlen` stops at the NUL that fgets writes. */
    let mut input_buffer = [0u8; MAX_INPUT_SIZE];

    /* Read operation number */
    if !fgets(&mut input_buffer, &mut stdin) {
        eprint!("Error reading operation\n");
        return ExitCode::from(1);
    }
    let operation = atoi(c_str(&input_buffer));

    /* Read parameter */
    if !fgets(&mut input_buffer, &mut stdin) {
        eprint!("Error reading parameter\n");
        return ExitCode::from(1);
    }
    let param = atoi(c_str(&input_buffer));

    /* Read decision string */
    if !fgets(&mut input_buffer, &mut stdin) {
        eprint!("Error reading decision string\n");
        return ExitCode::from(1);
    }

    /* Remove trailing newline if present */
    let mut len = strlen(&input_buffer);
    if len > 0 && input_buffer[len - 1] == b'\n' {
        input_buffer[len - 1] = 0;
        len -= 1;
    }

    /* Call the library function */
    let result = process_decisions(&input_buffer, len, operation, param);

    /* Print result to stdout */
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let _ = write!(stdout, "{}\n", result);
    let _ = stdout.flush();

    ExitCode::SUCCESS
}

/// Equivalent of `fgets(buf, buf.len(), stdin)`.
///
/// At most `buf.len() - 1` bytes are stored; reading stops after a
/// newline (which is kept) or at end of input. A NUL terminator is
/// written after the last stored byte. Returns `false` where C would
/// return `NULL`, i.e. when end-of-file or an error occurs before any
/// byte is read.
fn fgets<R: Read>(buf: &mut [u8], input: &mut R) -> bool {
    let capacity = buf.len();
    if capacity == 0 {
        return false;
    }

    let mut i = 0usize;
    let mut byte = [0u8; 1];
    while i + 1 < capacity {
        match input.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf[i] = byte[0];
                i += 1;
                if byte[0] == b'\n' {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    if i == 0 {
        /* EOF or error with nothing read: fgets returns NULL. */
        return false;
    }

    buf[i] = 0;
    true
}

/// Length of the NUL-terminated string held in `buf` (C `strlen`).
fn strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(pos) => pos,
        None => buf.len(),
    }
}

/// The NUL-terminated string held in `buf`, without the terminator.
fn c_str(buf: &[u8]) -> &[u8] {
    &buf[..strlen(buf)]
}

/// Equivalent of glibc's `atoi`, which is `(int)strtol(s, NULL, 10)`:
/// leading whitespace and an optional sign are skipped, digits are
/// consumed, out-of-range values saturate at `LONG_MAX`/`LONG_MIN` and
/// the result is then truncated to `int`.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;

    while i < s.len()
        && matches!(s[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
    {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let mut acc: i64 = 0;
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        if !overflowed {
            let digit = i64::from(s[i] - b'0');
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
        i += 1;
    }

    let value: i64 = if overflowed {
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
