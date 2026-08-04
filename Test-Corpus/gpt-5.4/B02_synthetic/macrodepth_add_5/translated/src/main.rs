mod mdcore;
mod mdmacros;

use mdcore::{helper_call, helper_ptr, use_generated, G_OP, G_OP_NAME};
use mdmacros::{accum_op, init_for_op, op_fn, OP_NAME, REPEAT};

fn main() {
    let mut args = std::env::args();
    let prog = args.next().unwrap_or_else(|| "driver".to_string());
    let a = match args.next().and_then(|s| s.parse::<i32>().ok()) {
        Some(v) => v,
        None => {
            eprintln!("usage: {} A B", prog);
            std::process::exit(2);
        }
    };
    let b = match args.next().and_then(|s| s.parse::<i32>().ok()) {
        Some(v) => v,
        None => {
            eprintln!("usage: {} A B", prog);
            std::process::exit(2);
        }
    };

    let r_call = op_fn(a, b);
    let mut acc = init_for_op();
    mdmacros::run_loop(&mut acc, REPEAT);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let g = G_OP(a, b);

    println!("op={} call={} acc={} g.call={}", G_OP_NAME, r_call, acc, g);
    println!("summary={}", r_call + acc + x1 + x2 + x3 + g);

    let _ = OP_NAME;
    let _ = accum_op;
}
