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

mod mdcore;
mod mdmacros;

use std::ffi::c_int;
use std::io::Write;

use mdcore::{helper_call, helper_ptr, use_generated, G_OP, G_OP_NAME};
use mdmacros::{cstdio::printf, op_apply, run_loop_from_init, REPEAT};

/// Re-implementation of C `atoi(3)`, which glibc defines as
/// `(int) strtol(nptr, NULL, 10)`: leading white space is skipped, an optional
/// sign is consumed, decimal digits are accumulated (saturating at `LONG_MIN` /
/// `LONG_MAX`) and the resulting `long` is truncated to `int`.  Anything that
/// does not parse yields `0`.
fn atoi(s: &[u8]) -> c_int {
    let mut idx = 0usize;

    // isspace(): ' ', '\t', '\n', '\v', '\f', '\r'
    while idx < s.len() && (s[idx] == b' ' || (s[idx] >= 0x09 && s[idx] <= 0x0d)) {
        idx += 1;
    }

    let mut negative = false;
    if idx < s.len() && (s[idx] == b'+' || s[idx] == b'-') {
        negative = s[idx] == b'-';
        idx += 1;
    }

    let mut acc: i64 = 0;
    let mut overflow = false;
    while idx < s.len() && s[idx].is_ascii_digit() {
        let digit = i64::from(s[idx] - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        idx += 1;
    }

    if overflow {
        // strtol() clamps to LONG_MIN / LONG_MAX, atoi() then truncates.
        return if negative {
            i64::MIN as c_int
        } else {
            i64::MAX as c_int
        };
    }

    let value = if negative { -acc } else { acc };
    value as c_int
}

fn main() {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    if argc < 3 {
        // fprintf(stderr, "usage: %s A B\n", argv[0]);
        let prog: &[u8] = if argc > 0 {
            os_bytes(&argv[0])
        } else {
            b"" as &[u8]
        };
        let stderr = std::io::stderr();
        let mut stderr = stderr.lock();
        let _ = stderr.write_all(b"usage: ");
        let _ = stderr.write_all(prog);
        let _ = stderr.write_all(b" A B\n");
        let _ = stderr.flush();
        std::process::exit(2);
    }

    let a = atoi(os_bytes(&argv[1]));
    let b = atoi(os_bytes(&argv[2]));

    let r_call: c_int = op_apply(a, b);
    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let acc: c_int = run_loop_from_init();

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    // int g = G_OP(a, b);  -- read through the mutable global, exactly like C.
    let g = unsafe { G_OP }(a, b);

    // printf("op=%s call=%d acc=%d g.call=%d\n", G_OP_NAME, r_call, acc, g);
    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            G_OP_NAME,
            r_call,
            acc,
            g,
        );
    }
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    // printf("summary=%d\n", r_call + acc + x1 + x2 + x3 + g);
    unsafe {
        printf(c"summary=%d\n".as_ptr(), summary);
    }

    // return 0;
}

/// Raw bytes of a command line argument (no UTF-8 validation, matching C).
fn os_bytes(s: &std::ffi::OsString) -> &[u8] {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        s.as_os_str().as_bytes()
    }
    #[cfg(not(unix))]
    {
        // Fall back to the lossy UTF-8 view on non-unix targets.
        s.as_os_str().to_str().unwrap_or("").as_bytes()
    }
}
