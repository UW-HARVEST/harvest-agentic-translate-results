use std::ffi::{c_char, c_int};

type OpFn = extern "C" fn(c_int, c_int) -> c_int;

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Operation {
    Add,
    Sub,
    Mul,
}

// Cargo permits more than one feature to be enabled. Multiplication takes
// precedence over subtraction, which takes precedence over addition.
#[cfg(feature = "mul")]
const SELECTED_OP: Operation = Operation::Mul;
#[cfg(all(not(feature = "mul"), feature = "sub"))]
const SELECTED_OP: Operation = Operation::Sub;
#[cfg(all(not(feature = "mul"), not(feature = "sub"), feature = "add"))]
const SELECTED_OP: Operation = Operation::Add;
#[cfg(all(not(feature = "mul"), not(feature = "sub"), not(feature = "add")))]
const SELECTED_OP: Operation = Operation::Add;

// With no REPEAT feature, preserve CMake's default value of 5. If several
// values are enabled, the greatest enabled value wins deterministically.
#[cfg(feature = "7")]
pub(crate) const REPEAT: c_int = 7;
#[cfg(all(not(feature = "7"), feature = "6"))]
pub(crate) const REPEAT: c_int = 6;
#[cfg(all(not(feature = "7"), not(feature = "6"), feature = "5"))]
pub(crate) const REPEAT: c_int = 5;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    feature = "4"
))]
pub(crate) const REPEAT: c_int = 4;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    feature = "3"
))]
pub(crate) const REPEAT: c_int = 3;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    feature = "2"
))]
pub(crate) const REPEAT: c_int = 2;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    feature = "1"
))]
pub(crate) const REPEAT: c_int = 1;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    not(feature = "1"),
    feature = "0"
))]
pub(crate) const REPEAT: c_int = 0;
#[cfg(all(
    not(feature = "7"),
    not(feature = "6"),
    not(feature = "5"),
    not(feature = "4"),
    not(feature = "3"),
    not(feature = "2"),
    not(feature = "1"),
    not(feature = "0")
))]
pub(crate) const REPEAT: c_int = 5;

const ADD_NAME: &[u8; 4] = b"add\0";
const SUB_NAME: &[u8; 4] = b"sub\0";
const MUL_NAME: &[u8; 4] = b"mul\0";

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

const fn selected_fn() -> OpFn {
    match SELECTED_OP {
        Operation::Add => op_add,
        Operation::Sub => op_sub,
        Operation::Mul => op_mul,
    }
}

const fn selected_name() -> *const c_char {
    let bytes = match SELECTED_OP {
        Operation::Add => ADD_NAME,
        Operation::Sub => SUB_NAME,
        Operation::Mul => MUL_NAME,
    };
    bytes.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub static mut G_OP: OpFn = selected_fn();

#[unsafe(no_mangle)]
pub static mut G_OP_NAME: *const c_char = selected_name();

pub(crate) fn selected_operation(a: c_int, b: c_int) -> c_int {
    selected_fn()(a, b)
}

fn initial_accumulator() -> c_int {
    match SELECTED_OP {
        Operation::Add | Operation::Sub => 0,
        Operation::Mul => 1,
    }
}

fn step(acc: c_int, i: c_int) -> c_int {
    match SELECTED_OP {
        Operation::Add => acc.wrapping_add(i),
        Operation::Sub => acc.wrapping_sub(i),
        Operation::Mul => acc.wrapping_mul(i.wrapping_add(1)),
    }
}

pub(crate) fn repeated_accumulator(n: c_int) -> c_int {
    let mut acc = initial_accumulator();
    let mut i = 0;
    while i < n {
        acc = step(acc, i);
        i += 1;
    }
    acc
}

fn generated_accumulator(n: c_int) -> c_int {
    if (0..=6).contains(&n) {
        repeated_accumulator(n)
    } else {
        initial_accumulator()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_call(a: c_int, b: c_int) -> c_int {
    let result = selected_operation(a, b);
    let acc = repeated_accumulator(REPEAT);
    unsafe {
        printf(c"helper.call=%d helper.acc=%d\n".as_ptr(), result, acc);
    }
    result.wrapping_add(acc)
}

#[unsafe(no_mangle)]
pub extern "C" fn helper_ptr(a: c_int, b: c_int) -> c_int {
    let function = selected_fn();
    let result = function(a, b);
    unsafe {
        printf(c"helper.ptr=%d\n".as_ptr(), result);
    }
    result
}

#[unsafe(no_mangle)]
pub extern "C" fn use_generated(n: c_int) -> c_int {
    let result = generated_accumulator(n);
    unsafe {
        printf(c"gen.acc=%d\n".as_ptr(), result);
    }
    result
}
