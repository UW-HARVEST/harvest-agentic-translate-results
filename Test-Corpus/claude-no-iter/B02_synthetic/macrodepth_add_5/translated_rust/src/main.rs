// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/mdmain.c

use std::ffi::c_int;
use std::process::ExitCode;

use driver::{helper_call, helper_ptr, run_loop, use_generated, INIT_FOR, OP_FN_PTR, OP_NAME_STR, REPEAT};

/// Mimic C `atoi`: parse the leading optional sign and as many digits as
/// possible (with C `int` wrap-around semantics) and return 0 if no digits.
fn c_atoi(s: &str) -> c_int {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace for ASCII whitespace,
    // which is what C's atoi does for the typical C locale).
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') {
        i += 1;
    }
    let mut sign: c_int = 1;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => i += 1,
            b'-' => {
                sign = -1;
                i += 1;
            }
            _ => {}
        }
    }
    let mut acc: c_int = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as c_int;
        acc = acc.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    acc.wrapping_mul(sign)
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    let argc = argv.len();

    if argc < 3 {
        eprintln!("usage: {} A B", argv[0]);
        return ExitCode::from(2);
    }

    let a = c_atoi(&argv[1]);
    let b = c_atoi(&argv[2]);

    let r_call = OP_FN_PTR(a, b);
    let mut acc: c_int = INIT_FOR;
    run_loop(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = OP_FN_PTR(a, b); // G_OP points to OP_FN_PTR

    println!(
        "op={} call={} acc={} g.call={}",
        OP_NAME_STR, r_call, acc, g
    );

    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={}", summary);
    ExitCode::from(0)
}
