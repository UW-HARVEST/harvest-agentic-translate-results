// Translation of c_src/src/mdmain.c — driver binary.

use driver::{
    accum_op, helper_call, helper_ptr, init_for_op, run_loop, use_generated, G_OP, OP_NAME, REPEAT,
};
use std::process::ExitCode;

/// C-style atoi: parse leading optional sign and decimal digits, ignore the
/// rest, and return 0 if nothing parses.
fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // Skip leading ASCII whitespace as atoi does.
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut acc: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as i32;
        acc = acc.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    if neg {
        acc.wrapping_neg()
    } else {
        acc
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} A B", args[0]);
        return ExitCode::from(2);
    }
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    // r_call = (OP_FN(OP))(a, b)
    let r_call = G_OP(a, b);

    // acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let mut acc = init_for_op();
    run_loop(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = G_OP(a, b);

    println!("op={} call={} acc={} g.call={}", OP_NAME, r_call, acc, g);
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    println!("summary={}", summary);

    // Suppress dead-code warning for accum_op when not used elsewhere.
    let _ = accum_op;
    ExitCode::SUCCESS
}
