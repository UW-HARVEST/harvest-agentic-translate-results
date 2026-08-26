// Rust translation of c_src/src/mdmain.c.
//
// Builds the same binary behavior:
//   if (argc < 3): print usage to stderr and exit 2
//   else parse a, b with atoi; run helpers and print results.

use std::env;
use std::ffi::c_int;
use std::process::ExitCode;

use driver::{
    helper_call, helper_ptr, run_loop, selected_op_fn, selected_op_init, use_generated,
    G_OP_NAME, REPEAT, SELECTED_OP_NAME,
};

/// Mirror C's `atoi`: parse leading optional whitespace, optional sign, then
/// digits; stop at the first non-digit and return whatever was parsed.  No
/// errors, no overflow checks.
fn atoi(s: &str) -> c_int {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len()
        && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
    {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut n: c_int = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        n = n
            .wrapping_mul(10)
            .wrapping_add((bytes[i] - b'0') as c_int);
        i += 1;
    }
    if neg { n.wrapping_neg() } else { n }
}

fn main() -> ExitCode {
    let argv: Vec<String> = env::args().collect();
    let argc = argv.len();

    if argc < 3 {
        let prog = argv.first().map(String::as_str).unwrap_or("");
        eprintln!("usage: {} A B", prog);
        return ExitCode::from(2);
    }

    let a = atoi(&argv[1]);
    let b = atoi(&argv[2]);

    // Direct call through the same compile-time-selected op function:
    //   int r_call = (OP_FN(OP))(a, b);
    let op_fn = selected_op_fn();
    let r_call: c_int = op_fn(a, b);

    // int acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT);
    let mut acc: c_int = selected_op_init();
    run_loop(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);

    // g = G_OP(a, b)
    // SAFETY: G_OP_NAME / G_OP are statics initialized at module load time.
    let g_op_fn = unsafe { std::ptr::read(&driver::G_OP) };
    let g: c_int = g_op_fn(a, b);

    // Read G_OP_NAME (a *const c_char) for the printout — but we already have
    // the same string as a Rust &'static str, so just use it.
    let _ = &G_OP_NAME; // keep a reference so the symbol is exported.
    let op_name = SELECTED_OP_NAME;

    println!(
        "op={} call={} acc={} g.call={}",
        op_name, r_call, acc, g
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
