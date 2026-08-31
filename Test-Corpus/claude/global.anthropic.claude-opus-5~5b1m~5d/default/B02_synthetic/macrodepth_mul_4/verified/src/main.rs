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

//! Rust equivalent of `mdmain.c` (the `driver` binary entry point).
//!
//! The shared modules are included directly (rather than via the `cdylib`
//! crate, which produces no Rust-linkable `rlib`) so this binary is fully
//! self-contained while reusing the exact same source as the library.

mod mdcore;
mod mdmacros;

use core::ffi::c_int;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

use mdcore::{helper_call, helper_ptr, use_generated, G_OP, G_OP_NAME};
use mdmacros::{op_fn, step, INIT_FOR, REPEAT};

/// Whitespace recognized by the C locale `isspace`, as used by `strtol`/`atoi`.
fn c_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful reimplementation of C `atoi`, which on glibc is
/// `(int) strtol(nptr, NULL, 10)`:
///
/// * leading whitespace is skipped,
/// * an optional `+`/`-` sign is consumed,
/// * decimal digits are accumulated until a non-digit is seen,
/// * on overflow `strtol` clamps to `LONG_MIN`/`LONG_MAX` (64-bit) and the
///   result is then truncated to `int`,
/// * a string with no digits yields `0`.
fn c_atoi(bytes: &[u8]) -> c_int {
    let n = bytes.len();
    let mut i = 0usize;

    while i < n && c_isspace(bytes[i]) {
        i += 1;
    }

    let mut neg = false;
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }

    // Accumulate in i64 (== C `long` on the target platform); clamp on overflow
    // to mirror strtol, then truncate to `int`.
    let mut acc: i64 = 0;
    let mut clamped = false;
    while i < n && bytes[i].is_ascii_digit() {
        if !clamped {
            let d = (bytes[i] - b'0') as i64;
            let next = acc
                .checked_mul(10)
                .and_then(|v| if neg { v.checked_sub(d) } else { v.checked_add(d) });
            match next {
                Some(v) => acc = v,
                None => {
                    acc = if neg { i64::MIN } else { i64::MAX };
                    clamped = true;
                }
            }
        }
        i += 1;
    }

    acc as c_int
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();

    if args.len() < 3 {
        // fprintf(stderr, "usage: %s A B\n", argv[0]);
        let prog: &[u8] = args.first().map(|s| s.as_bytes()).unwrap_or(b"");
        let stderr = std::io::stderr();
        let mut h = stderr.lock();
        let _ = h.write_all(b"usage: ");
        let _ = h.write_all(prog);
        let _ = h.write_all(b" A B\n");
        std::process::exit(2);
    }

    let a = c_atoi(args[1].as_bytes());
    let b = c_atoi(args[2].as_bytes());

    let r_call = op_fn(a, b);

    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let mut acc = INIT_FOR;
    let mut i: c_int = 0;
    while i < REPEAT {
        acc = step(acc, i);
        i += 1;
    }

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    // `int g = G_OP(a, b);` — reads the mutable global, exactly like the C.
    // SAFETY: single-threaded; nothing has written `G_OP` since load time.
    let g = unsafe { G_OP }(a, b);

    // `printf("op=%s ...", G_OP_NAME, ...)` — also reads the mutable global.
    // SAFETY: as above; the pointer refers to a `'static` NUL-terminated literal.
    let op_name = unsafe { core::ffi::CStr::from_ptr(G_OP_NAME) };
    println!(
        "op={} call={} acc={} g.call={}",
        op_name.to_string_lossy(),
        r_call,
        acc,
        g
    );
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={}", summary);
}
