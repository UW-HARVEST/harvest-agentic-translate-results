use std::env;

pub fn op_add(a: i32, b: i32) -> i32 { a + b }
pub fn op_sub(a: i32, b: i32) -> i32 { a - b }
pub fn op_mul(a: i32, b: i32) -> i32 { a * b }

pub const OP: &str = env!("OP", "add");
pub const REPEAT: usize = {
    const S: &str = env!("REPEAT", "5");
    parse_usize(S)
};

const fn parse_usize(s: &str) -> usize {
    let mut i = 0;
    let mut val = 0usize;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        let b = bytes[i];
        if b >= b'0' && b <= b'9' {
            val = val * 10 + (b - b'0') as usize;
        }
        i += 1;
    }
    val
}

pub fn step_add(acc: &mut i32, i: i32) { *acc += i; }
pub fn step_sub(acc: &mut i32, i: i32) { *acc -= i; }
pub fn step_mul(acc: &mut i32, i: i32) { *acc *= i + 1; }

pub fn init_add() -> i32 { 0 }
pub fn init_sub() -> i32 { 0 }
pub fn init_mul() -> i32 { 1 }

pub fn run_loop_add(acc: &mut i32, n: usize) {
    for i in 0..n {
        step_add(acc, i as i32);
    }
}

pub fn run_loop_sub(acc: &mut i32, n: usize) {
    for i in 0..n {
        step_sub(acc, i as i32);
    }
}

pub fn run_loop_mul(acc: &mut i32, n: usize) {
    for i in 0..n {
        step_mul(acc, i as i32);
    }
}

pub fn accum_add(n: usize) -> i32 {
    let mut acc = init_add();
    run_loop_add(&mut acc, n);
    acc
}

pub fn accum_sub(n: usize) -> i32 {
    let mut acc = init_sub();
    run_loop_sub(&mut acc, n);
    acc
}

pub fn accum_mul(n: usize) -> i32 {
    let mut acc = init_mul();
    run_loop_mul(&mut acc, n);
    acc
}

pub static G_OP_NAME: &str = OP;

pub fn g_op(a: i32, b: i32) -> i32 {
    match OP {
        "sub" => op_sub(a, b),
        "mul" => op_mul(a, b),
        _ => op_add(a, b),
    }
}

pub fn op_fn(a: i32, b: i32) -> i32 {
    match OP {
        "sub" => op_sub(a, b),
        "mul" => op_mul(a, b),
        _ => op_add(a, b),
    }
}

pub fn init_for() -> i32 {
    match OP {
        "sub" => init_sub(),
        "mul" => init_mul(),
        _ => init_add(),
    }
}

pub fn run_loop(acc: &mut i32, n: usize) {
    match OP {
        "sub" => run_loop_sub(acc, n),
        "mul" => run_loop_mul(acc, n),
        _ => run_loop_add(acc, n),
    }
}

pub fn accum_fn(n: usize) -> i32 {
    match OP {
        "sub" => accum_sub(n),
        "mul" => accum_mul(n),
        _ => accum_add(n),
    }
}

pub fn helper_call(a: i32, b: i32) -> i32 {
    let r = op_fn(a, b);
    let mut acc = init_for();
    run_loop(&mut acc, REPEAT);
    println!("helper.call={} helper.acc={}", r, acc);
    r + acc
}

pub fn helper_ptr(a: i32, b: i32) -> i32 {
    let r = op_fn(a, b);
    println!("helper.ptr={}", r);
    r
}

pub fn use_generated(n: usize) -> i32 {
    let r = accum_fn(n);
    println!("gen.acc={}", r);
    r
}
