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
//! The C program is a CWE-562 ("Return of Stack Variable Address") test case.
//! See `helper_bad` below for how the original's undefined behavior is mirrored.

use std::io::{Read, Write};

/// Mirror of the C `printLine`: `printf("%s\n", line)` guarded by a NULL check.
///
/// The C signature takes `const char *`, so "no string at all" (NULL) is a
/// representable input; `Option<&str>` is the safe-Rust equivalent.
fn print_line(out: &mut dyn Write, line: Option<&str>) {
    if let Some(line) = line {
        // `printf("%s\n", ...)`. glibc rewrites this to `puts`, which emits the
        // same bytes: the string followed by a single newline.
        let _ = write!(out, "{}\n", line);
    }
}

/// Mirror of the C `helperBad`, which returns the address of a local array:
///
/// ```c
/// static char *helperBad()
/// {
///     char charString[] = "helperBad string";
///     return charString;   // dangling pointer -- undefined behavior
/// }
/// ```
///
/// This is the injected defect, and it is deliberately NOT fixed here. Returning
/// a dangling pointer is undefined behavior, so there is no "correct" value to
/// reproduce -- only the behavior the C actually exhibits once compiled.
///
/// GCC diagnoses this as `-Wreturn-local-addr` and folds the return value to a
/// null pointer. The generated `helperBad` is literally `mov $0x0, %eax; ret`
/// (verified against the compiled reference binary at both `-O0` and `-O2`).
/// `printLine`'s NULL check therefore succeeds and the bad path prints nothing
/// at all -- not even a newline. `None` reproduces that byte-for-byte while
/// keeping this translation free of unsafe code and dangling references.
fn helper_bad() -> Option<&'static str> {
    None
}

/// Mirror of the C `bad`.
fn bad(out: &mut dyn Write) {
    print_line(out, helper_bad());
}

/// Mirror of the C `helperGood1`, which returns a pointer to a `static` array.
/// The storage outlives the call, so the pointer stays valid -- modeled as a
/// `&'static str`.
fn helper_good1() -> Option<&'static str> {
    Some("helperGood1 string")
}

/// Mirror of the C `good`.
fn good(out: &mut dyn Write) {
    print_line(out, helper_good1());
}

/// Byte classification matching C's `isspace` in the default "C" locale, which
/// is the set of characters `scanf` skips before a conversion.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Equivalent of glibc's `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion and `None` on input failure
/// (EOF before any non-whitespace) or matching failure (no digits). On `None`
/// the caller must leave its variable untouched, exactly as `scanf` does.
///
/// Reads one byte at a time so that no more input is consumed than `%d` would,
/// and so leading whitespace -- including newlines -- is skipped, per the
/// documented `scanf` behavior of reading across lines.
fn scanf_i32(input: &mut dyn Read) -> Option<i32> {
    // One-byte lookahead, since `%d` must push back the first byte that cannot
    // extend the number.
    let mut pending: Option<u8> = None;
    let mut next = |pending: &mut Option<u8>| -> Option<u8> {
        if let Some(b) = pending.take() {
            return Some(b);
        }
        let mut buf = [0u8; 1];
        loop {
            return match input.read(&mut buf) {
                Ok(0) => None,
                Ok(_) => Some(buf[0]),
                Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => None,
            };
        }
    };

    // Skip leading whitespace.
    let mut b = loop {
        match next(&mut pending) {
            Some(b) if is_c_space(b) => continue,
            Some(b) => break b,
            // EOF with nothing consumed: input failure.
            None => return None,
        }
    };

    // Optional sign.
    let negative = match b {
        b'-' | b'+' => {
            let negative = b == b'-';
            match next(&mut pending) {
                Some(nb) => b = nb,
                // Sign then EOF: matching failure, no digits.
                None => return None,
            }
            negative
        }
        _ => false,
    };

    if !b.is_ascii_digit() {
        // A sign or other character not followed by a digit is a matching
        // failure. `scanf` would push the offending byte back onto the stream;
        // that is unobservable here because `main` performs no further input.
        return None;
    }

    // glibc accumulates into a `long` with `strtol` semantics: out-of-range
    // input saturates at LONG_MAX / LONG_MIN. The result is then stored through
    // an `int *`, truncating the low 32 bits. That truncation is observable
    // here, because e.g. 4294967296 truncates to 0 and thus takes the `else`
    // branch. Verified against the reference binary.
    let mut acc: i64 = 0;
    let mut saturated = false;
    loop {
        let digit = i64::from(b - b'0');
        match acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(digit))
        {
            Some(v) => acc = v,
            None => saturated = true,
        }

        match next(&mut pending) {
            Some(nb) if nb.is_ascii_digit() => b = nb,
            Some(_nb) => {
                // First byte that cannot extend the number would be pushed
                // back; unobservable, as `main` reads no further input.
                break;
            }
            None => break,
        }
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        // `acc` is non-negative here and `-acc` cannot overflow.
        -acc
    } else {
        acc
    };

    // Store through `int *`: keep the low 32 bits.
    Some(value as i32)
}

/// Restore the default `SIGPIPE` disposition.
///
/// A C program inherits `SIG_DFL` for `SIGPIPE`, so it is killed by the signal
/// if stdout is a pipe whose reader closes early (observable as exit status
/// 141). The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, which would
/// instead turn the failed write into an ignored error and exit 0. Undoing that
/// keeps the process's externally visible behavior identical to the C.
///
/// This is the one `unsafe` block in the translation; it is required because
/// signal disposition is not reachable from safe `std`.
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

    let stdin = std::io::stdin();
    let mut input = stdin.lock();

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    // `int x = 0; scanf("%d", &x);` -- on conversion failure `x` keeps its
    // initial value of 0, so the `else` (bad) branch runs. The return value of
    // `scanf` is ignored by the C, and is ignored here too.
    let mut x: i32 = 0;
    if let Some(v) = scanf_i32(&mut input) {
        x = v;
    }

    if x != 0 {
        good(&mut out);
    } else {
        bad(&mut out);
    }

    let _ = out.flush();
    std::process::exit(0);
}
