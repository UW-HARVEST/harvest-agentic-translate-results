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

use std::io::{self, BufRead, BufReader, Write};
use std::process::ExitCode;

use driver::process_decisions;

const MAX_INPUT_SIZE: usize = 1024;

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    let mut input_buffer = [0u8; MAX_INPUT_SIZE];
    let operation: i32;
    let param: i32;
    let result: i32;

    /* Read operation number */
    if !fgets(&mut reader, &mut input_buffer) {
        eprint!("Error reading operation\n");
        return ExitCode::from(1);
    }
    operation = atoi(&input_buffer);

    /* Read parameter */
    if !fgets(&mut reader, &mut input_buffer) {
        eprint!("Error reading parameter\n");
        return ExitCode::from(1);
    }
    param = atoi(&input_buffer);

    /* Read decision string */
    if !fgets(&mut reader, &mut input_buffer) {
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
    result = process_decisions(Some(&input_buffer[..len]), len, operation, param);

    /* Print result to stdout */
    print!("{}\n", result);
    let _ = io::stdout().flush();

    ExitCode::from(0)
}

/// Length of the NUL-terminated string held in `buf`, like C's `strlen`.
fn strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&c| c == 0).unwrap_or(buf.len())
}

/// Emulates `fgets(buf, buf.len(), stdin)`.
///
/// Reads at most `buf.len() - 1` bytes, stopping after the first newline
/// (which is kept in the buffer) or at end of file, and NUL-terminates.
/// Returns `false` (C's NULL) when end of file is reached with no bytes read,
/// or on a read error. Bytes past a long line stay in the stream for the next
/// call, exactly as with `fgets`.
fn fgets<R: BufRead>(reader: &mut R, buf: &mut [u8]) -> bool {
    let cap = buf.len() - 1;
    let mut n = 0usize;

    while n < cap {
        let (copied, stop) = {
            let avail = match reader.fill_buf() {
                Ok(a) => a,
                Err(_) => return false,
            };
            if avail.is_empty() {
                /* End of file. */
                if n == 0 {
                    return false;
                }
                break;
            }
            let take = if avail.len() < cap - n {
                avail.len()
            } else {
                cap - n
            };
            match avail[..take].iter().position(|&c| c == b'\n') {
                Some(pos) => {
                    buf[n..n + pos + 1].copy_from_slice(&avail[..pos + 1]);
                    (pos + 1, true)
                }
                None => {
                    buf[n..n + take].copy_from_slice(&avail[..take]);
                    (take, false)
                }
            }
        };
        reader.consume(copied);
        n += copied;
        if stop {
            break;
        }
    }

    buf[n] = 0;
    true
}

/// Emulates glibc's `atoi`, which is `(int) strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is accepted, digits are
/// consumed until a non-digit, overflow saturates at `long` bounds and the
/// result is truncated to `int`.
fn atoi(buf: &[u8]) -> i32 {
    let s = &buf[..strlen(buf)];
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
