use std::ffi::{c_char, c_int};

use crate::config::{OPERATION, Operation, REPEAT};

pub(crate) type OpFn = extern "C" fn(c_int, c_int) -> c_int;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn op_add(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn op_sub(a: c_int, b: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn op_mul(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

const fn selected_op() -> OpFn {
    match OPERATION {
        Operation::Add => op_add,
        Operation::Sub => op_sub,
        Operation::Mul => op_mul,
    }
}

const fn initial_value() -> c_int {
    match OPERATION {
        Operation::Add | Operation::Sub => 0,
        Operation::Mul => 1,
    }
}

fn step(acc: c_int, i: c_int) -> c_int {
    match OPERATION {
        Operation::Add => acc.wrapping_add(i),
        Operation::Sub => acc.wrapping_sub(i),
        Operation::Mul => acc.wrapping_mul(i.wrapping_add(1)),
    }
}

fn accumulate_steps(n: c_int) -> c_int {
    let mut acc = initial_value();
    let mut i = 0;
    while i < n {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

pub(crate) fn configured_accumulator() -> c_int {
    accumulate_steps(REPEAT)
}

fn generated_accumulator(n: c_int) -> c_int {
    match n {
        0..=6 => accumulate_steps(n),
        _ => initial_value(),
    }
}

pub(crate) fn selected_call(a: c_int, b: c_int) -> c_int {
    selected_op()(a, b)
}

#[unsafe(no_mangle)]
pub static mut G_OP: OpFn = selected_op();

const ADD_NAME: &[u8] = b"add\0";
const SUB_NAME: &[u8] = b"sub\0";
const MUL_NAME: &[u8] = b"mul\0";

const fn selected_name() -> *const c_char {
    match OPERATION {
        Operation::Add => ADD_NAME.as_ptr().cast(),
        Operation::Sub => SUB_NAME.as_ptr().cast(),
        Operation::Mul => MUL_NAME.as_ptr().cast(),
    }
}

#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = selected_name();

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let r = selected_call(a, b);
    let acc = configured_accumulator();
    unsafe {
        printf(c"helper.call=%d helper.acc=%d\n".as_ptr(), r, acc);
    }
    r.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let fp = selected_op();
    let r = fp(a, b);
    unsafe {
        printf(c"helper.ptr=%d\n".as_ptr(), r);
    }
    r
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let r = generated_accumulator(n);
    unsafe {
        printf(c"gen.acc=%d\n".as_ptr(), r);
    }
    r
}
