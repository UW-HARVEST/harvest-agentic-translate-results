// Copyright 2025 MIT Lincoln Laboratory
// Rust translation preserving exact behavior of c_src/src/lib.c

use std::ffi::c_char;
use std::os::raw::c_int;
use std::ptr;

// time_t on Linux is i64
#[allow(non_camel_case_types)]
type time_t = i64;

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
enum Operation {
    OpAdd = 1,
    OpMultiply = 2,
    OpSubtract = 3,
    OpDivide = 4,
    OpModulo = 5,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
#[allow(dead_code)]
enum StatusCode {
    StatusSuccess = 0,
    StatusError = -1,
    StatusWarning = 1,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ComputationResult {
    value: c_int,
    timestamp: time_t,
    status: StatusCode,
}

type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

unsafe extern "C" {
    fn time(t: *mut time_t) -> time_t;
    fn calloc(nmemb: usize, size: usize) -> *mut std::ffi::c_void;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn is_valid_operation(op_char: c_char) -> bool {
    // C: char valid = op_char && (op_char >= '1' && op_char <= '5');
    // op_char != 0 && in range
    let valid = (op_char != 0) && (op_char >= b'1' as c_char && op_char <= b'5' as c_char);
    valid
}

fn get_operation_priority(op: c_int) -> c_int {
    // priority = op * 10
    op.wrapping_mul(10)
}

extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    // C signed division semantics; emulate with wrapping_div which matches for non-overflow,
    // but a/b for INT_MIN / -1 is UB in C; use wrapping_div to avoid Rust panic.
    a.wrapping_div(b)
}

extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

fn select_operation(op: c_int) -> MathOperation {
    match op {
        1 => add_operation,
        2 => multiply_operation,
        3 => subtract_operation,
        4 => divide_operation,
        5 => modulo_operation,
        _ => add_operation,
    }
}

fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    unsafe {
        time(&mut current_time as *mut time_t);
    }
    current_time = current_time >> 29;
    current_time
}

fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe {
        calloc(count as usize, std::mem::size_of::<ComputationResult>()) as *mut ComputationResult
    }
}

unsafe fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
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
        (*((*history).offset(idx))).value = result;
        (*((*history).offset(idx))).timestamp = get_computation_timestamp();
        (*((*history).offset(idx))).status = StatusCode::StatusSuccess;
        *history_count += 1;
    }

    result
}

// Static state preserved across calls (matches C's static locals in mathop)
static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mathop(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut validation_char: c_char = (param1.rem_euclid_c(128)) as c_char;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as c_char;
    }
    let _ = validation_char; // mirror C's unused-after assignment

    // C: (Operation)((param3 % 5) + 1)
    let selected_op: c_int = (param3.wrapping_rem(5)).wrapping_add(1);

    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        &raw mut COMPUTATION_HISTORY,
        &raw mut HISTORY_COUNT,
    );

    // C: (Operation)(((param4 + 1) % 5) + 1)
    let second_op: c_int = (param4.wrapping_add(1).wrapping_rem(5)).wrapping_add(1);
    let mut final_result = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        &raw mut COMPUTATION_HISTORY,
        &raw mut HISTORY_COUNT,
    );

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();

    // C: (int)(computation_time % 100)
    let time_modifier: c_int = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    let fmt1 = b"Computation performed at timestamp: %ld\n\0";
    let fmt2 = b"Operation priority: %d\n\0";
    let fmt3 = b"History entries: %d\n\0";
    let fmt4 = b"Final result: %d\n\0";

    printf(fmt1.as_ptr() as *const c_char, computation_time as core::ffi::c_long);
    printf(fmt2.as_ptr() as *const c_char, operation_priority);
    printf(fmt3.as_ptr() as *const c_char, HISTORY_COUNT);
    printf(fmt4.as_ptr() as *const c_char, final_result);

    final_result
}

// Helper trait to mirror C's `%` semantics for signed integers (truncated toward zero).
// Rust's `%` on signed ints already truncates toward zero, matching C.
trait CRem {
    fn rem_euclid_c(self, rhs: Self) -> Self;
}
impl CRem for c_int {
    fn rem_euclid_c(self, rhs: c_int) -> c_int {
        // Use wrapping_rem to mirror C and avoid panics on overflow edge case.
        self.wrapping_rem(rhs)
    }
}
