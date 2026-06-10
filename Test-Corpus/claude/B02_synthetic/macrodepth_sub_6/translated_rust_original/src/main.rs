// Translated from c_src/src/mdmain.c
// Reproduces the C binary's stdout byte-for-byte.

use std::ffi::c_int;
use std::process::ExitCode;

use driver::{driver_init_for, driver_op, driver_op_name, driver_run_loop, helper_call, helper_ptr, use_generated, REPEAT};

// Mirror C atoi semantics for the inputs we expect (int parsing, leading
// whitespace + optional sign + digits; non-numeric tail is ignored). For our
// driver we accept simple decimal integers, falling back to 0 on parse error
// to match atoi's behavior on bad input.
fn c_atoi(s: &str) -> c_int {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == 0x0B || bytes[i] == 0x0C) {
        i += 1;
    }
    let mut sign: c_int = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut val: c_int = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as c_int);
        i += 1;
    }
    val.wrapping_mul(sign)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        let prog = args.first().map(String::as_str).unwrap_or("driver");
        eprintln!("usage: {} A B", prog);
        return ExitCode::from(2);
    }
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    let r_call = driver_op(a, b);
    let mut acc = driver_init_for();
    driver_run_loop(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = driver_op(a, b);

    println!("op={} call={} acc={} g.call={}", driver_op_name(), r_call, acc, g);
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={}", summary);
    ExitCode::from(0)
}
