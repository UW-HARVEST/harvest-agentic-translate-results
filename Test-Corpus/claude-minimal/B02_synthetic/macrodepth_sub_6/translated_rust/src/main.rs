// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

mod mdcore;
mod mdmacros;

use std::env;
use std::process::ExitCode;

use crate::mdcore::{g_op, g_op_name, helper_call, helper_ptr, use_generated};
use crate::mdmacros::{init_for, op_fn, run_loop, OP, REPEAT};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        let prog = args.first().map(|s| s.as_str()).unwrap_or("driver");
        eprintln!("usage: {} A B", prog);
        return ExitCode::from(2);
    }

    // `atoi` in C silently returns 0 for non-numeric input.
    let a: i32 = args[1].parse().unwrap_or(0);
    let b: i32 = args[2].parse().unwrap_or(0);

    let r_call = (op_fn(OP))(a, b);
    let mut acc = init_for(OP);
    run_loop(OP, &mut acc, REPEAT);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = (g_op())(a, b);

    println!(
        "op={} call={} acc={} g.call={}",
        g_op_name(),
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

    ExitCode::SUCCESS
}
