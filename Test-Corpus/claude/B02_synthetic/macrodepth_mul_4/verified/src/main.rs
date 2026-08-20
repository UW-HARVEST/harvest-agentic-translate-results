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

//! Rust translation of `src/mdmain.c` (the `driver` executable).
//!
//! The library crate is a `cdylib`, so the binary compiles the same modules
//! directly instead of linking against it.

mod mdcore;
mod mdmacros;

use std::ffi::{c_int, OsString};
use std::process::ExitCode;

use mdcore::{helper_call, helper_ptr, use_generated, OP_FN, G_OP, G_OP_NAME};
use mdmacros::{init_for_op, run_loop, OP_NAME, REPEAT};

/// Byte view of a command line argument, matching what C's `argv` holds.
#[cfg(unix)]
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    arg.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn arg_bytes(arg: &OsString) -> Vec<u8> {
    arg.to_string_lossy().into_owned().into_bytes()
}

/// Emulates glibc's `atoi`, which is `(int)strtol(nptr, NULL, 10)`:
/// leading whitespace is skipped, an optional sign is honoured, digits are
/// consumed until a non-digit, out-of-range values saturate at `LONG_MIN` /
/// `LONG_MAX` and the result is then truncated to `int`.
fn c_atoi(s: &[u8]) -> c_int {
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
    let mut overflowed = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let digit = i64::from(s[i] - b'0');
        if !overflowed {
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

    value as c_int
}

/// Stand-in for C's `printf` to `stdout`: `mdmain.c` discards the return value,
/// so a failing write must be ignored rather than panicking (`print!` would
/// abort the process and change the exit status, e.g. on `/dev/full`).
#[inline]
fn c_printf(args: std::fmt::Arguments<'_>) {
    use std::io::Write;
    let _ = std::io::stdout().lock().write_fmt(args);
}

/// C: `fprintf(stderr, "usage: %s A B\n", argv[0]);`
///
/// `argv[0]` is emitted as raw bytes -- a program path need not be valid UTF-8
/// and C copies it verbatim, so lossy conversion would change the output.
fn usage(prog: &[u8]) {
    use std::io::Write;
    let mut msg = Vec::with_capacity(prog.len() + 16);
    msg.extend_from_slice(b"usage: ");
    msg.extend_from_slice(prog);
    msg.extend_from_slice(b" A B\n");
    let _ = std::io::stderr().lock().write_all(&msg);
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let argc = args.len();

    if argc < 3 {
        // With `argc == 0` there is no `argv[0]` at all; glibc's `%s` then
        // contributes no bytes, which an empty slice reproduces.
        let prog = args.first().map(arg_bytes).unwrap_or_default();
        usage(&prog);
        return ExitCode::from(2);
    }

    let a = c_atoi(&arg_bytes(&args[1]));
    let b = c_atoi(&arg_bytes(&args[2]));

    let r_call = (OP_FN)(a, b);
    let acc = run_loop(init_for_op());

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = (G_OP)(a, b);

    // C: `printf("op=%s ...", G_OP_NAME, ...)`. `G_OP_NAME` is initialised to
    // `STR(OP)` at file scope and never reassigned, so it always holds exactly
    // `OP_NAME`; printing `OP_NAME` is byte-identical and needs no raw pointer
    // dereference. (The two are checked to agree by
    // `tests/differential.rs::b15_g_op_name_bytes`.)
    let _ = &G_OP_NAME;
    c_printf(format_args!(
        "op={} call={} acc={} g.call={}\n",
        OP_NAME, r_call, acc, g
    ));
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    c_printf(format_args!("summary={}\n", summary));
    ExitCode::SUCCESS
}
