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

use std::io::{Read, Write};

/// Restore the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a write
/// to a pipe with no reader returns `EPIPE` instead of killing the process. The
/// C program inherits the default disposition, so it is terminated by signal 13
/// in that situation. Without this, the two programs disagree on exit status
/// whenever stdout is a closed pipe (C: killed by SIGPIPE, Rust: exit 0).
#[cfg(unix)]
fn restore_default_sigpipe() {
    // Linked directly rather than via the `libc` crate to keep the crate
    // dependency-free. `signal` is in libc, which is always linked on unix.
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// Mirrors the C `printLine` helper. Kept for structural fidelity with the
/// original translation unit even though `main` never reaches it.
#[allow(dead_code)]
fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{}", line);
    }
}

/// `printIntLine(int)` -> `printf("%d\n", intNumber)`
fn print_int_line(int_number: i32) {
    // Write through a locked handle so ordering matches C's stdout stream.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "{}\n", int_number);
}

/// Translation of the C `bad()`.
///
/// The original allocates only `alloca(10)` (10 *bytes*) yet stores ten
/// `int`s (40 bytes) into it, overflowing the stack allocation. That write is
/// undefined behavior in C; in practice the loop copies the zero-initialized
/// `source` array and `data[0]` reads back as 0. We reproduce the *observable*
/// behavior (printing `data[0]`) with a correctly sized safe buffer rather
/// than recreating the out-of-bounds write, since the printed value is the
/// only thing the program exposes.
fn bad() {
    // `alloca(10)` in the C original -- deliberately undersized there.
    let mut data: Vec<i32> = vec![0; 10];
    {
        let source: [i32; 10] = [0; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        print_int_line(data[0]);
    }
}

/// Translation of the C `good()`: `alloca(10 * sizeof(int))`, properly sized.
fn good() {
    let mut data: Vec<i32> = vec![0; 10];
    {
        let source: [i32; 10] = [0; 10];
        let mut i: usize = 0;
        while i < 10 {
            data[i] = source[i];
            i += 1;
        }
        print_int_line(data[0]);
    }
}

/// Emulates a single `scanf("%d", &x)` conversion against stdin.
///
/// Matches C semantics: leading whitespace (including newlines) is skipped,
/// an optional sign may follow, then one or more decimal digits. On matching
/// failure or EOF the destination is left untouched, exactly as `scanf` does.
/// Reading is byte-at-a-time so no more input than necessary is consumed.
/// Overflow follows glibc, which accumulates into a `long` saturating at
/// `LONG_MAX`/`LONG_MIN` and then truncates the result to `int`.
fn scanf_i32(dest: &mut i32) -> i32 {
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();
    let mut byte = [0u8; 1];

    let mut next = |handle: &mut dyn Read| -> Option<u8> {
        match handle.read(&mut byte) {
            Ok(1) => Some(byte[0]),
            _ => None,
        }
    };

    // Skip leading whitespace, as the %d directive does.
    let mut c = loop {
        match next(&mut handle) {
            None => return -1, // EOF before any conversion -> scanf returns EOF
            Some(ch) => {
                if !matches!(ch, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
                    break ch;
                }
            }
        }
    };

    // Optional sign.
    let mut negative = false;
    if c == b'+' || c == b'-' {
        negative = c == b'-';
        match next(&mut handle) {
            // A sign was consumed, so EOF here is a *matching* failure, not an
            // input failure: glibc returns 0, not EOF. Verified against glibc.
            None => return 0,
            Some(ch) => c = ch,
        }
    }

    if !c.is_ascii_digit() {
        return 0; // matching failure; `dest` left unmodified
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    loop {
        let digit = i64::from(c - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        match next(&mut handle) {
            None => break,
            Some(ch) => {
                if ch.is_ascii_digit() {
                    c = ch;
                } else {
                    // Non-digit terminator is pushed back by scanf; the program
                    // performs no further reads, so dropping it is equivalent.
                    break;
                }
            }
        }
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

    // glibc stores the accumulated long truncated to int.
    *dest = value as i32;
    1
}

fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;
    let _ = scanf_i32(&mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    let _ = std::io::stdout().flush();
}
