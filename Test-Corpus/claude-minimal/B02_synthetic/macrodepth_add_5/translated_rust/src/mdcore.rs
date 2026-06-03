// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use crate::mdmacros::{accum, init_for, op_fn, run_loop, OP, REPEAT};

/// Function pointer global, equivalent to `int (*G_OP)(int,int)` in the C
/// source.  Set at startup based on the configured `OP`.
pub fn g_op() -> fn(i32, i32) -> i32 {
    op_fn(OP)
}

/// Equivalent of `const char *G_OP_NAME` in C.
pub fn g_op_name() -> &'static str {
    OP
}

pub fn helper_call(a: i32, b: i32) -> i32 {
    let r = (op_fn(OP))(a, b);
    let mut acc = init_for(OP);
    run_loop(OP, &mut acc, REPEAT);
    println!("helper.call={} helper.acc={}", r, acc);
    r.wrapping_add(acc)
}

pub fn helper_ptr(a: i32, b: i32) -> i32 {
    let fp: fn(i32, i32) -> i32 = op_fn(OP);
    let r = fp(a, b);
    println!("helper.ptr={}", r);
    r
}

pub fn use_generated(n: i32) -> i32 {
    let r = accum(OP, n);
    println!("gen.acc={}", r);
    r
}
