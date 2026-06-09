// Copyright 2025 MIT Lincoln Laboratory
// Translation of mdmain.c to Rust.

use std::env;
use std::ffi::c_int;
use std::process::ExitCode;

use driver::{
    helper_call, helper_ptr, op_selected, init_for_selected, run_loop_selected, use_generated,
    G_OP, OP_NAME, REPEAT,
};

fn atoi(s: &str) -> c_int {
    // Mimic libc atoi: parse leading optional whitespace, optional sign, then digits.
    // Stops at first non-digit. Returns 0 on no digits.
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // Skip whitespace as isspace() would.
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }
    let mut neg = false;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => i += 1,
            b'-' => {
                neg = true;
                i += 1;
            }
            _ => {}
        }
    }
    let mut n: c_int = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        n = n.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as c_int);
        i += 1;
    }
    if neg {
        n = n.wrapping_neg();
    }
    n
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} A B", args[0]);
        return ExitCode::from(2);
    }
    let a = atoi(&args[1]);
    let b = atoi(&args[2]);

    let r_call = op_selected(a, b);
    let mut acc = init_for_selected();
    run_loop_selected(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = G_OP(a, b);

    println!(
        "op={} call={} acc={} g.call={}",
        OP_NAME, r_call, acc, g
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
