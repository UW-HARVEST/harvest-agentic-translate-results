// Translation of c_src/src/mdmain.c
//
// The C build links mdmain.c against mdcore.c directly. The Rust library is a
// `cdylib` (so it cannot be linked as a Rust dependency), therefore the binary
// compiles the same modules into itself, exactly like the C driver does.

#![allow(dead_code)]

#[path = "mdcore.rs"]
mod mdcore;
#[path = "mdmacros.rs"]
mod mdmacros;

use std::ffi::c_int;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;

/// `atoi(3)`: optional leading whitespace, optional sign, then decimal digits;
/// parsing stops at the first non-digit. No error is reported.
///
/// glibc implements it as `(int)strtol(...)`, so out-of-range input saturates
/// at `long` bounds and is then truncated to `int`.
fn atoi(bytes: &[u8]) -> c_int {
    let mut idx = 0usize;

    while idx < bytes.len() && matches!(bytes[idx], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
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
        let digit = i64::from(bytes[idx] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        idx += 1;
    }

    let value = if saturated {
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

fn main() {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = argv.len();

    if argc < 3 {
        let program = argv
            .first()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut stderr = std::io::stderr();
        let _ = write!(stderr, "usage: {} A B\n", program);
        let _ = stderr.flush();
        std::process::exit(2);
    }

    let a = atoi(argv[1].as_bytes());
    let b = atoi(argv[2].as_bytes());

    // int r_call = (OP_FN(OP))(a, b);
    let r_call = (mdmacros::OP_FN)(a, b);

    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let acc = mdmacros::run_loop(mdmacros::INIT);

    let x1 = mdcore::helper_call(a, b);
    let x2 = mdcore::helper_ptr(a, b);
    let x3 = mdcore::use_generated(mdmacros::REPEAT);
    let g = (mdcore::G_OP)(a, b);

    println!(
        "op={} call={} acc={} g.call={}",
        mdmacros::op_name_str(),
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
