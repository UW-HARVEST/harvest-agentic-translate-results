//! Translation of `c_src/src/mdmain.c` — the `driver` executable.
//!
//! `[lib] crate-type = ["cdylib"]` means this binary cannot link against the
//! library target, so it compiles the same modules directly, exactly like the
//! CMake target compiles both `mdcore.c` and `mdmain.c` into `driver`.

// `mdcore` mirrors a C translation unit: it defines the whole operation family
// and the exported helpers whether or not the driver calls every one of them.
#![allow(dead_code)]

mod mdcore;
mod mdmacros;

use core::ffi::{CStr, c_int};
use std::io::Write;
use std::process::ExitCode;

use mdcore::{G_OP, G_OP_NAME, OP_FN, helper_call, helper_ptr, use_generated};
use mdmacros::{INIT, REPEAT, run_loop};

/// `atoi` from `<stdlib.h>`.
///
/// glibc implements it as `(int) strtol(nptr, NULL, 10)`: leading whitespace is
/// skipped, an optional sign is consumed, digits are accumulated, trailing junk
/// is ignored and there is no error reporting. On overflow `strtol` saturates to
/// `LONG_MIN`/`LONG_MAX` (64-bit here) and the cast to `int` truncates — e.g.
/// `atoi("99999999999999999999")` yields `-1`. Reproduced as-is.
fn c_atoi(bytes: &[u8]) -> c_int {
    let mut idx = 0;

    // isspace()
    while idx < bytes.len()
        && matches!(
            bytes[idx],
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r'
        )
    {
        idx += 1;
    }

    let mut negative = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        negative = bytes[idx] == b'-';
        idx += 1;
    }

    let mut acc: i64 = 0;
    let mut saturated = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        if !saturated {
            let digit = i64::from(bytes[idx] - b'0');
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        idx += 1;
    }

    let value: i64 = if saturated {
        if negative { i64::MIN } else { i64::MAX }
    } else if negative {
        -acc
    } else {
        acc
    };

    value as c_int
}

/// Raw argument bytes, so operands and `argv[0]` round-trip exactly like in C.
fn arg_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

fn main() -> ExitCode {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    // if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }
    if argc < 3 {
        let prog = argv.first().map(arg_bytes).unwrap_or_default();
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"usage: ");
        let _ = stderr.write_all(&prog);
        let _ = stderr.write_all(b" A B\n");
        let _ = stderr.flush();
        return ExitCode::from(2);
    }

    let a = c_atoi(&arg_bytes(&argv[1]));
    let b = c_atoi(&arg_bytes(&argv[2]));

    let r_call = OP_FN(a, b);

    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let acc = run_loop(INIT);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = G_OP(a, b);

    // printf("op=%s call=%d acc=%d g.call=%d\n", G_OP_NAME, ...)
    let op_name = unsafe { CStr::from_ptr(G_OP_NAME.0) }.to_string_lossy();
    println!("op={op_name} call={r_call} acc={acc} g.call={g}");

    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={summary}");

    ExitCode::SUCCESS
}
