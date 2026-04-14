use crate::mdmacros::{accum_op, init_for_op, op_fn, run_loop, OP_NAME};

pub const G_OP: fn(i32, i32) -> i32 = op_fn;
pub const G_OP_NAME: &str = OP_NAME;

pub fn helper_call(a: i32, b: i32) -> i32 {
    let r = op_fn(a, b);
    let mut acc = init_for_op();
    run_loop(&mut acc, crate::mdmacros::REPEAT);
    println!("helper.call={} helper.acc={}", r, acc);
    r + acc
}

pub fn helper_ptr(a: i32, b: i32) -> i32 {
    let fp: fn(i32, i32) -> i32 = op_fn;
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

pub fn use_generated(n: i32) -> i32 {
    let r = accum_op(n);
    println!("gen.acc={}", r);
    r
}
