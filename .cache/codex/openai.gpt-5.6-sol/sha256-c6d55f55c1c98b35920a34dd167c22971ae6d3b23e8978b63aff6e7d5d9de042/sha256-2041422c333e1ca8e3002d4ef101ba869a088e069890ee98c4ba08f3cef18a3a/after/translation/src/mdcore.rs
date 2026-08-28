use std::ffi::{c_char, c_int};

use crate::config::{OP, Operation, REPEAT, generated_accumulator, run_unrolled};

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

const fn selected_function() -> extern "C" fn(c_int, c_int) -> c_int {
    match OP {
        Operation::Add => op_add,
        Operation::Sub => op_sub,
        Operation::Mul => op_mul,
    }
}

#[unsafe(no_mangle)]
pub static mut G_OP: extern "C" fn(c_int, c_int) -> c_int = selected_function();

#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = OP.name_c();

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let result = OP.apply(a, b);
    let accumulator = run_unrolled(OP, REPEAT);
    unsafe {
        printf(
            c"helper.call=%d helper.acc=%d\n".as_ptr(),
            result,
            accumulator,
        );
    }
    result.wrapping_add(accumulator)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let function = selected_function();
    let result = function(a, b);
    unsafe {
        printf(c"helper.ptr=%d\n".as_ptr(), result);
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let result = generated_accumulator(OP, n);
    unsafe {
        printf(c"gen.acc=%d\n".as_ptr(), result);
    }
    result
}
