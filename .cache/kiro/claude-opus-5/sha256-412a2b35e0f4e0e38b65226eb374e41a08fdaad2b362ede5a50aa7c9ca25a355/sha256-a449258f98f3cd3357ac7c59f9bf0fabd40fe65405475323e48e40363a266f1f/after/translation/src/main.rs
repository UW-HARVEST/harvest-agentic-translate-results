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
//! Takes two arguments, a base and an exponent, and prints `base^exponent`.
//!
//! The C original observes three libc side effects that Rust's standard library
//! does not expose, so each is reimplemented in a dedicated module:
//!
//! * [`strtod`] - C's `strtod`, including `endptr` placement and the exact
//!   conditions under which it reports `ERANGE`.
//! * [`cpow`] - `pow` plus the `EDOM` / `ERANGE` that glibc's `pow` leaves in
//!   `errno`; the numeric result itself comes from `f64::powf`, which lowers to
//!   the same libm routine the C program calls.
//! * [`cfmt`] - `printf("%.2f", ...)`, which differs from Rust's `{:.2}` for
//!   NaN.
//!
//! Argument handling mirrors C's `argv`: the error messages echo the raw
//! argument bytes back through `%s`, so arguments are handled as bytes rather
//! than as `String`s (an argument need not be valid UTF-8).

mod cfmt;
mod cpow;
mod strtod;

use cfmt::format_f2;
use cpow::Errno;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

/// `fprintf(stderr, ...)`: stderr is unbuffered in C and write errors there are
/// not checked, so failures are discarded here too.
fn stderr_write(bytes: &[u8]) {
    let mut err = std::io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

fn stdout_write(bytes: &[u8]) {
    let mut out = std::io::stdout();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

/// Builds `<prefix><arg><suffix>` for the messages that interpolate a raw
/// argument with `%s`.
fn message(prefix: &[u8], arg: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(prefix.len() + arg.len() + suffix.len());
    msg.extend_from_slice(prefix);
    msg.extend_from_slice(arg);
    msg.extend_from_slice(suffix);
    msg
}

/// Converts one argument, printing the matching diagnostic and exiting 1 on
/// failure, in C's order: the `ERANGE` check precedes the trailing character
/// check.
///
/// The two arguments produce differently worded diagnostics, so both message
/// templates are passed in.
fn convert(arg: &[u8], range_prefix: &[u8], invalid_prefix: &[u8]) -> f64 {
    // C stops at the first NUL; `argv` strings cannot contain one, but slicing
    // there keeps the two programs equivalent even if one somehow appeared.
    let s = match arg.iter().position(|&b| b == 0) {
        Some(i) => &arg[..i],
        None => arg,
    };

    let conv = strtod::strtod(s);

    if conv.erange {
        stderr_write(&message(range_prefix, arg, b"'\n"));
        std::process::exit(1);
    }
    // `*endptr != '\0'`: anything left unconsumed before the terminator.  An
    // input that converts nothing leaves `endptr == nptr`, so `""` yields 0.0
    // and is accepted - the C program does the same.
    if conv.consumed < s.len() {
        stderr_write(&message(invalid_prefix, arg, b"'\n"));
        std::process::exit(1);
    }

    conv.value
}

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();

    if argv.len() != 3 {
        // With an empty `argv`, C reads `argv[0] == NULL`, which glibc's
        // `printf("%s")` renders as `(null)`.
        let prog: &[u8] = argv.first().map_or(&b"(null)"[..], |a| a.as_slice());
        stderr_write(&message(b"Usage: ", prog, b" base exponent\n"));
        std::process::exit(1);
    }

    // Convert base
    let base = convert(
        &argv[1],
        b"Range error while converting base '",
        b"Invalid numeric input for base: '",
    );

    // Convert exponent
    let exponent = convert(
        &argv[2],
        b"Range error while converting exponent '",
        b"Invalid numeric input for exponent: '",
    );

    // Calculate power
    let (result, errno) = cpow::pow_with_errno(base, exponent);
    if errno == Errno::Edom {
        let msg = format!(
            "Domain error: pow({}, {}) is undefined in the real number domain.\n",
            format_f2(base),
            format_f2(exponent)
        );
        stderr_write(msg.as_bytes());
        std::process::exit(1);
    } else if errno == Errno::Erange {
        let msg = format!(
            "Range error: pow({}, {}) caused overflow or underflow.\n",
            format_f2(base),
            format_f2(exponent)
        );
        stderr_write(msg.as_bytes());
        std::process::exit(1);
    }

    stdout_write(format!("Result: {}\n", format_f2(result)).as_bytes());
    std::process::exit(0);
}
