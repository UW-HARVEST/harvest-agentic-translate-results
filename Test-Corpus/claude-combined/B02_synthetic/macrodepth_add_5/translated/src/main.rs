// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior is preserved exactly.

use driver::{driver_init_for, driver_run_loop, driver_selected_op, helper_call, helper_ptr,
             use_generated, G_OP, G_OP_NAME, REPEAT};
use std::ffi::CStr;
use std::os::raw::c_int;

/// C-style atoi: parse a leading optional sign followed by ASCII digits, stop
/// at the first non-digit, return 0 if no digits found. Matches glibc atoi
/// semantics (which behave like strtol(s, NULL, 10) but truncated to int).
fn c_atoi(s: &str) -> c_int {
    let bytes = s.as_bytes();
    let mut i = 0;
    // skip leading whitespace (atoi follows isspace())
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r')
    {
        i += 1;
    }
    let mut sign: c_int = 1;
    if i < bytes.len() {
        if bytes[i] == b'-' {
            sign = -1;
            i += 1;
        } else if bytes[i] == b'+' {
            i += 1;
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

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() < 3 {
        let prog = argv.first().map(String::as_str).unwrap_or("driver");
        eprintln!("usage: {} A B", prog);
        std::process::exit(2);
    }
    let a: c_int = c_atoi(&argv[1]);
    let b: c_int = c_atoi(&argv[2]);

    let r_call: c_int = (driver_selected_op())(a, b);
    let mut acc: c_int = driver_init_for();
    driver_run_loop(&mut acc);

    let x1: c_int = helper_call(a, b);
    let x2: c_int = helper_ptr(a, b);
    let x3: c_int = use_generated(REPEAT);

    // SAFETY: G_OP and G_OP_NAME are initialized by the .init_array
    // constructor before main runs.
    let g: c_int = unsafe {
        let f = G_OP.expect("G_OP must be initialized");
        f(a, b)
    };
    let name = unsafe {
        CStr::from_ptr(G_OP_NAME)
            .to_str()
            .expect("G_OP_NAME must be valid UTF-8")
    };

    println!(
        "op={} call={} acc={} g.call={}",
        name, r_call, acc, g
    );
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={}", summary);
}
