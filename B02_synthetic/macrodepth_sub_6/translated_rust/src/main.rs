use std::env;
use std::process;

fn op_add(a: i32, b: i32) -> i32 { a + b }

/// accum_add: switch-dispatched accumulator (mirrors DEFINE_ACCUM(add))
fn accum_add(n: i32) -> i32 {
    let mut acc: i32 = 0; // INIT_add
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
    let mut acc: i32 = 0; // INIT_add
    // RUN_LOOP(add, acc, 5) -> REP5
    acc += 0; acc += 1; acc += 2; acc += 3; acc += 4;
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
    let a = atoi(&args[1]);
    let b = atoi(&args[2]);

    let r_call = op_add(a, b);
    let mut acc: i32 = 0; // INIT_add
    // RUN_LOOP(add, acc, 5) -> REP5
    acc += 0; acc += 1; acc += 2; acc += 3; acc += 4;

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(5); // REPEAT=5
    let g = op_add(a, b);      // G_OP(a, b)

    println!("op=add call={} acc={} g.call={}", r_call, acc, g);
    println!("summary={}", r_call + acc + x1 + x2 + x3 + g);
}

/// Mimics C atoi: parse leading integer, return 0 on failure
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    if s.is_empty() { return 0; }
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
    let mut val: i32 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            val = val.wrapping_mul(10).wrapping_add(d as i32);
        } else {
            break;
        }
    }
    if neg { val.wrapping_neg() } else { val }
}
