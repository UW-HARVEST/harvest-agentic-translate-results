mod mdcore;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} A B", args[0]);
        process::exit(2);
    }

    let a: i32 = args[1].parse().unwrap_or(0);
    let b: i32 = args[2].parse().unwrap_or(0);

    let r_call = mdcore::op_add(a, b);
    let mut acc = mdcore::INIT_ADD;
    
    mdcore::step_add(&mut acc, 0);
    mdcore::step_add(&mut acc, 1);
    mdcore::step_add(&mut acc, 2);
    mdcore::step_add(&mut acc, 3);
    mdcore::step_add(&mut acc, 4);

    let x1 = mdcore::helper_call(a, b);
    let x2 = mdcore::helper_ptr(a, b);
    let x3 = mdcore::use_generated(mdcore::REPEAT);
    let g = (mdcore::G_OP)(a, b);

    println!("op={} call={} acc={} g.call={}", mdcore::G_OP_NAME, r_call, acc, g);
    println!("summary={}", r_call + acc + x1 + x2 + x3 + g);
}
