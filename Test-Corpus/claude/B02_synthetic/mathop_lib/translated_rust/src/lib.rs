// Copyright 2025 MIT Lincoln Laboratory
// Translation of c_src/src/lib.c to Rust.
// Preserves the exact behavior (including bugs/quirks) of the original C code.

use std::ffi::c_char;
use std::os::raw::{c_int, c_long};

// Operation enum matching the C `Operation` enum (values 1..=5)
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

// StatusCode enum matching the C `StatusCode` enum
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum StatusCode {
    Success = 0,
    Error = -1,
    Warning = 1,
}

// time_t on Linux is a 64-bit signed integer
pub type TimeT = i64;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: TimeT,
    pub status: StatusCode,
}

// Function pointer type matching C's `int (*)(int, int, int)`
pub type MathOperation = extern "C" fn(c_int, c_int, c_int) -> c_int;

extern "C" {
    fn time(tloc: *mut TimeT) -> TimeT;
    fn calloc(nmemb: usize, size: usize) -> *mut std::ffi::c_void;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    // C: char valid = op_char && (op_char >= '1' && op_char <= '5');
    let valid = (op_char != 0) && (op_char >= b'1' as c_char && op_char <= b'5' as c_char);
    valid
}

#[no_mangle]
pub extern "C" fn get_operation_priority(op: Operation) -> c_int {
    let priority = (op as c_int) * 10;
    priority
}

#[no_mangle]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a + b
}

#[no_mangle]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a * b
}

#[no_mangle]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a - b
}

#[no_mangle]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a / b
}

#[no_mangle]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a % b
}

#[no_mangle]
pub extern "C" fn select_operation(op: Operation) -> MathOperation {
    match op {
        Operation::Add => add_operation,
        Operation::Multiply => multiply_operation,
        Operation::Subtract => subtract_operation,
        Operation::Divide => divide_operation,
        Operation::Modulo => modulo_operation,
    }
}

// Convert a c_int into the C `Operation` enum.
// In C, the cast `(Operation)x` accepts any int; we mimic by treating the
// raw int as the enum's discriminant. Values outside 1..=5 are unreachable
// for `select_operation` because of the default case in the C switch.
fn op_from_int(v: c_int) -> Operation {
    match v {
        1 => Operation::Add,
        2 => Operation::Multiply,
        3 => Operation::Subtract,
        4 => Operation::Divide,
        5 => Operation::Modulo,
        _ => Operation::Add,
    }
}

#[no_mangle]
pub extern "C" fn get_computation_timestamp() -> TimeT {
    let mut current_time: TimeT = 0;
    unsafe {
        time(&mut current_time as *mut TimeT);
    }
    current_time = current_time >> 29;
    current_time
}

#[no_mangle]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe {
        calloc(count as usize, std::mem::size_of::<ComputationResult>()) as *mut ComputationResult
    }
}

#[no_mangle]
pub unsafe extern "C" fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func = select_operation(op);

    let result = math_func(a, b, 0);

    if (*history).is_null() {
        *history = allocate_results(10);
        *history_count = 0;
    }

    if *history_count < 10 {
        let idx = *history_count as isize;
        let entry = (*history).offset(idx);
        (*entry).value = result;
        (*entry).timestamp = get_computation_timestamp();
        (*entry).status = StatusCode::Success;
        *history_count += 1;
    }

    result
}

// Static state (matches the C `static` locals inside `mathop`).
static mut COMPUTATION_HISTORY: *mut ComputationResult = std::ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[no_mangle]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        let mut validation_char: c_char = (param1 % 128) as c_char;
        let is_valid = is_valid_operation(validation_char);

        if !is_valid {
            validation_char = b'1' as c_char;
        }
        // `validation_char` is computed but otherwise unused after this point;
        // mirror the C behavior of evaluating but not consuming the result.
        let _ = validation_char;

        let selected_op = op_from_int((param3 % 5) + 1);

        let operation_priority = get_operation_priority(selected_op);

        let intermediate_result = perform_computation_with_history(
            param1,
            param2,
            selected_op,
            &raw mut COMPUTATION_HISTORY,
            &raw mut HISTORY_COUNT,
        );

        let second_op = op_from_int(((param4 + 1) % 5) + 1);
        let mut final_result = perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            &raw mut COMPUTATION_HISTORY,
            &raw mut HISTORY_COUNT,
        );

        final_result += operation_priority;

        let computation_time = get_computation_timestamp();

        let time_modifier = (computation_time % 100) as c_int;
        final_result += time_modifier;

        let fmt1 = b"Computation performed at timestamp: %ld\n\0".as_ptr() as *const c_char;
        printf(fmt1, computation_time as c_long);

        let fmt2 = b"Operation priority: %d\n\0".as_ptr() as *const c_char;
        printf(fmt2, operation_priority);

        let fmt3 = b"History entries: %d\n\0".as_ptr() as *const c_char;
        printf(fmt3, HISTORY_COUNT);

        let fmt4 = b"Final result: %d\n\0".as_ptr() as *const c_char;
        printf(fmt4, final_result);

        final_result
    }
}
