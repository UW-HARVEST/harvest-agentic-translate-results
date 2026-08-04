// Translation of mdmain.c to Rust.
// Default OP=add, REPEAT=5 from CMakeLists.txt

use std::os::raw::c_int;
use std::process::ExitCode;

use driver::{helper_call, helper_ptr, init_for_add, op_add, run_loop_add_5, use_generated, REPEAT};

// Mimic C's atoi: parse a leading optional sign + digits; ignore anything else.
fn c_atoi(s: &str) -> c_int {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // Skip leading whitespace like C atoi
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == 0x0b || bytes[i] == 0x0c) {
        i += 1;
    }
    let mut sign: c_int = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: c_int = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as c_int;
        // Wrap like C does on overflow.
        result = result.wrapping_mul(10).wrapping_add(d);
        i += 1;
    }
    result.wrapping_mul(sign)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        let prog = args.first().map(|s| s.as_str()).unwrap_or("driver");
        eprintln!("usage: {} A B", prog);
        return ExitCode::from(2);
    }
    let a = c_atoi(&args[1]);
    let b = c_atoi(&args[2]);

    // r_call = (OP_FN(OP))(a, b) — for OP=add this is op_add
    let r_call = op_add(a, b);

    // acc = INIT_FOR(OP); RUN_LOOP(OP, acc, REPEAT)
    let mut acc = init_for_add();
    run_loop_add_5(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    // g = G_OP(a, b) — just call op_add (the value stored in G_OP)
    let g = op_add(a, b);

    println!(
        "op={} call={} acc={} g.call={}",
        driver::op_name_str(),
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
    ExitCode::from(0)
}
