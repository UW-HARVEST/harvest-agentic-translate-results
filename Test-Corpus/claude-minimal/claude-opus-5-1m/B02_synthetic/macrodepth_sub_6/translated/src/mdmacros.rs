// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.
//
// This module mirrors the compile-time configuration that the original C
// code received via CMake (`-DOP=<op> -DREPEAT=<n>`).  In Rust we expose
// the same selections through a small set of constants and helper
// functions which are dispatched at run time from the configured `OP`
// constant.  The CMake defaults (`OP=add`, `REPEAT=5`) are reproduced
// here.

/// Selected operation, mirroring the C `OP` macro.  Allowed values are
/// "add", "sub", and "mul".
pub const OP: &str = "add";

/// Selected loop repetition count, mirroring the C `REPEAT` macro.
pub const REPEAT: i32 = 5;

/* ---------- Operation implementations ---------- */

pub fn op_add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

pub fn op_sub(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

pub fn op_mul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

/// Equivalent to the C `OP_FN(op)` macro: pick the binary function for
/// the given operation name.
pub fn op_fn(op: &str) -> fn(i32, i32) -> i32 {
    match op {
        "add" => op_add,
        "sub" => op_sub,
        "mul" => op_mul,
        _ => panic!("unknown op: {}", op),
    }
}

/// Equivalent to the C `INIT_FOR(op)` macro.
pub fn init_for(op: &str) -> i32 {
    match op {
        "add" => 0,
        "sub" => 0,
        "mul" => 1,
        _ => panic!("unknown op: {}", op),
    }
}

/// One step of the unrolled loop.  Mirrors the `STEP_<op>` family of
/// macros.
#[inline]
pub fn step_op(op: &str, acc: &mut i32, i: i32) {
    match op {
        "add" => *acc = acc.wrapping_add(i),
        "sub" => *acc = acc.wrapping_sub(i),
        "mul" => *acc = acc.wrapping_mul(i.wrapping_add(1)),
        _ => panic!("unknown op: {}", op),
    }
}

/// Equivalent to the C `RUN_LOOP(op, acc, n)` macro, which selects an
/// unrolled `REPn` based on the (compile-time) `n` value.  In our Rust
/// translation the unrolling is simulated as a plain loop because both
/// REPEAT and OP are constants in this build, so the optimizer will
/// produce the same result.
#[inline]
pub fn run_loop(op: &str, acc: &mut i32, n: i32) {
    // Match the supported REPn values (0..=7) explicitly so behavior
    // tracks the C macro definitions.
    let upper = if (0..=7).contains(&n) { n } else { 0 };
    for i in 0..upper {
        step_op(op, acc, i);
    }
}

/// Equivalent to `DISPATCH_REP(op, acc, n)` from the C macros.
#[inline]
pub fn dispatch_rep(op: &str, acc: &mut i32, n: i32) {
    match n {
        0 => {}
        1 => {
            step_op(op, acc, 0);
        }
        2 => {
            step_op(op, acc, 0);
            step_op(op, acc, 1);
        }
        3 => {
            step_op(op, acc, 0);
            step_op(op, acc, 1);
            step_op(op, acc, 2);
        }
        4 => {
            step_op(op, acc, 0);
            step_op(op, acc, 1);
            step_op(op, acc, 2);
            step_op(op, acc, 3);
        }
        5 => {
            step_op(op, acc, 0);
            step_op(op, acc, 1);
            step_op(op, acc, 2);
            step_op(op, acc, 3);
            step_op(op, acc, 4);
        }
        6 => {
            step_op(op, acc, 0);
            step_op(op, acc, 1);
            step_op(op, acc, 2);
            step_op(op, acc, 3);
            step_op(op, acc, 4);
            step_op(op, acc, 5);
        }
        7 => {
            step_op(op, acc, 0);
            step_op(op, acc, 1);
            step_op(op, acc, 2);
            step_op(op, acc, 3);
            step_op(op, acc, 4);
            step_op(op, acc, 5);
            step_op(op, acc, 6);
        }
        _ => {}
    }
}

/// Macro-generated accumulator function:  `accum_<OP>(n)` in C.
pub fn accum(op: &str, n: i32) -> i32 {
    let mut acc = init_for(op);
    dispatch_rep(op, &mut acc, n);
    acc
}
