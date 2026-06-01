// Translation of c_src/src/lib.c to Rust.
// Preserves exact behavior, including the static computation history state
// and stdout output via libc::printf.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_long};

use libc::time_t;

// Operation enum values (matches C int enum).
pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

// StatusCode enum values.
pub const STATUS_SUCCESS: c_int = 0;
#[allow(dead_code)]
pub const STATUS_ERROR: c_int = -1;
#[allow(dead_code)]
pub const STATUS_WARNING: c_int = 1;

// typedef int (*MathOperation)(int, int, int);
pub type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

// typedef struct { int value; time_t timestamp; StatusCode status; } ComputationResult;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: time_t,
    pub status: c_int,
}

// bool is_valid_operation(char op_char)
#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    // C: char valid = op_char && (op_char >= '1' && op_char <= '5');
    //    return valid;
    // op_char is non-zero AND in range '1'..='5'.
    let nonzero = op_char != 0;
    let in_range = op_char >= b'1' as c_char && op_char <= b'5' as c_char;
    let valid: c_char = if nonzero && in_range { 1 } else { 0 };
    valid != 0
}

// int get_operation_priority(Operation op)
#[unsafe(no_mangle)]
pub extern "C" fn get_operation_priority(op: c_int) -> c_int {
    op.wrapping_mul(10)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    // C semantics: integer division (truncated toward zero). Note that
    // a / b in Rust panics on i32::MIN / -1, but C has undefined behavior
    // there. Match C using wrapping_div.
    a.wrapping_div(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

// MathOperation select_operation(Operation op)
#[unsafe(no_mangle)]
pub extern "C" fn select_operation(op: c_int) -> Option<MathOperation> {
    match op {
        OP_ADD => Some(add_operation),
        OP_MULTIPLY => Some(multiply_operation),
        OP_SUBTRACT => Some(subtract_operation),
        OP_DIVIDE => Some(divide_operation),
        OP_MODULO => Some(modulo_operation),
        _ => Some(add_operation),
    }
}

// time_t get_computation_timestamp()
#[unsafe(no_mangle)]
pub extern "C" fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    unsafe {
        libc::time(&mut current_time as *mut time_t);
    }
    // C: current_time = current_time >> 29;
    // time_t is signed on Linux; arithmetic right shift.
    current_time >> 29
}

// ComputationResult* allocate_results(int count)
#[unsafe(no_mangle)]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe {
        libc::calloc(count as libc::size_t, std::mem::size_of::<ComputationResult>())
            as *mut ComputationResult
    }
}

// int perform_computation_with_history(int a, int b, Operation op,
//                                      ComputationResult** history, int* history_count)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func = select_operation(op).expect("select_operation returns non-null");
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
        (*entry).status = STATUS_SUCCESS;
        *history_count += 1;
    }

    result
}

// Static mutable state for mathop. C semantics: a single global state.
// Match C's static-local behavior; not thread-safe (matches C source).
static mut COMPUTATION_HISTORY: *mut ComputationResult = std::ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

// int mathop(int param1, int param2, int param3, int param4)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mathop(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    // char validation_char = (char)(param1 % 128);
    // The cast to char can yield a signed value; we mirror that.
    let mut validation_char: c_char = (param1.wrapping_rem(128)) as c_char;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as c_char;
    }
    let _ = validation_char; // unused after reassignment, mirroring C

    // Operation selected_op = (Operation)((param3 % 5) + 1);
    let selected_op: c_int = (param3.wrapping_rem(5)).wrapping_add(1);

    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        &raw mut COMPUTATION_HISTORY,
        &raw mut HISTORY_COUNT,
    );

    // Operation second_op = (Operation)(((param4 + 1) % 5) + 1);
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

    // int time_modifier = (int)(computation_time % 100);
    let time_modifier: c_int = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    // printf("Computation performed at timestamp: %ld\n", (long)computation_time);
    let fmt1 = b"Computation performed at timestamp: %ld\n\0".as_ptr() as *const c_char;
    libc::printf(fmt1, computation_time as c_long);

    let fmt2 = b"Operation priority: %d\n\0".as_ptr() as *const c_char;
    libc::printf(fmt2, operation_priority);

    let fmt3 = b"History entries: %d\n\0".as_ptr() as *const c_char;
    libc::printf(fmt3, HISTORY_COUNT);

    let fmt4 = b"Final result: %d\n\0".as_ptr() as *const c_char;
    libc::printf(fmt4, final_result);

    final_result
}

// Keep the c_void import used.
#[allow(dead_code)]
fn _force_use(_p: *mut c_void) {}
