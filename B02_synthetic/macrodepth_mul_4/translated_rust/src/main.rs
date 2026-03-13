use std::env;
use std::process;

fn op_add(a: i32, b: i32) -> i32 { a + b }

/// Unrolled REP5 for add: acc += 0; acc += 1; acc += 2; acc += 3; acc += 4;
fn run_loop_add(acc: &mut i32) {
    *acc += 0;
    *acc += 1;
    *acc += 2;
    *acc += 3;
    *acc += 4;
}

/// DISPATCH_REP(add, acc, n) — switch on n
fn accum_add(n: i32) -> i32 {
    let mut acc: i32 = 0;
    match n {
        0 => {}
        1 => { acc += 0; }
        2 => { acc += 0; acc += 1; }
        3 => { acc += 0; acc += 1; acc += 2; }
        4 => { acc += 0; acc += 1; acc += 2; acc += 3; }
        5 => { acc += 0; acc += 1; acc += 2; acc += 3; acc += 4; }
        6 => { acc += 0; acc += 1; acc += 2; acc += 3; acc += 4; acc += 5; }
        _ => {}
    }
    acc
}

fn helper_call(a: i32, b: i32) -> i32 {
    let r = op_add(a, b);
    let mut acc: i32 = 0;
    run_loop_add(&mut acc);
    println!("helper.call={} helper.acc={}", r, acc);
    r + acc
}

fn helper_ptr(a: i32, b: i32) -> i32 {
    let fp: fn(i32, i32) -> i32 = op_add;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

fn use_generated(n: i32) -> i32 {
    let r = accum_add(n);
    println!("gen.acc={}", r);
    r
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} A B", args[0]);
        process::exit(2);
    }
    let a: i32 = args[1].parse().unwrap_or(0);
    let b: i32 = args[2].parse().unwrap_or(0);

    let r_call = op_add(a, b);
    let mut acc: i32 = 0;
    run_loop_add(&mut acc);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(5);
    let g = op_add(a, b); // G_OP is op_add

    println!("op=add call={} acc={} g.call={}", r_call, acc, g);
    println!("summary={}", r_call + acc + x1 + x2 + x3 + g);
}
