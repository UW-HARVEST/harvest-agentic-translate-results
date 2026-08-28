// Rust translation of c_src/src/lib.c
//
// Behaviour is reproduced exactly, including the quirks of the original C
// (no bug fixes): the shifted timestamp, the history cap that also stops the
// counter, the unused third argument of every math operation, and the
// `default:` fallthrough to addition for out-of-range operation values.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_long, c_void};

// ---------------------------------------------------------------------------
// libc bindings
//
// printf is used (rather than Rust's own stdout) so that buffering and the
// exact byte formatting match the C library verbatim; calloc is used so that
// `allocate_results` hands back memory a C caller may free().
// ---------------------------------------------------------------------------
extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn time(tloc: *mut time_t) -> time_t;
}

/// `time_t` on Linux/x86_64 (and every other 64-bit target of interest).
pub type time_t = i64;

// ---------------------------------------------------------------------------
// typedef enum { OP_ADD = 1, ... } Operation;
// typedef enum { STATUS_SUCCESS = 0, ... } StatusCode;
//
// The C code casts arbitrary (possibly negative, possibly out-of-range)
// integers to `Operation`, so the enums are modelled as plain `c_int`
// constants. This is ABI-identical to the C enums while remaining sound for
// values outside the enumerated set.
// ---------------------------------------------------------------------------
const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

const STATUS_SUCCESS: c_int = 0;

/// typedef struct { int value; time_t timestamp; StatusCode status; } ComputationResult;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: time_t,
    pub status: c_int,
}

/// typedef int (*MathOperation)(int, int, int);
pub type MathOperation = extern "C" fn(c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    // char valid = op_char && (op_char >= '1' && op_char <= '5');
    let valid: c_char = if op_char != 0 && (op_char >= b'1' as c_char && op_char <= b'5' as c_char) {
        1
    } else {
        0
    };
    valid != 0
}

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
    a.wrapping_div(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn select_operation(op: c_int) -> MathOperation {
    match op {
        OP_ADD => add_operation,
        OP_MULTIPLY => multiply_operation,
        OP_SUBTRACT => subtract_operation,
        OP_DIVIDE => divide_operation,
        OP_MODULO => modulo_operation,
        _ => add_operation,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    unsafe { time(&mut current_time) };
    // Arithmetic right shift, as for a signed C integer.
    current_time >> 29
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    // A negative `count` sign-extends to a huge size_t, exactly as in C, and
    // calloc then returns NULL.
    unsafe {
        calloc(
            count as usize,
            std::mem::size_of::<ComputationResult>(),
        ) as *mut ComputationResult
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_computation_with_history(
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
        let entry = (*history).offset(*history_count as isize);
        (*entry).value = result;
        (*entry).timestamp = get_computation_timestamp();
        (*entry).status = STATUS_SUCCESS;
        *history_count += 1;
    }

    result
}

// static ComputationResult* computation_history = NULL;
// static int history_count = 0;
static mut COMPUTATION_HISTORY: *mut ComputationResult = std::ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let history_ptr: *mut *mut ComputationResult = &raw mut COMPUTATION_HISTORY;
    let count_ptr: *mut c_int = &raw mut HISTORY_COUNT;

    // char validation_char = (char)(param1 % 128);
    let mut validation_char: c_char = (param1.wrapping_rem(128)) as c_char;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as c_char;
    }
    // `validation_char` is never read again in the original C either.
    let _ = validation_char;

    let selected_op: c_int = param3.wrapping_rem(5).wrapping_add(1);

    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = unsafe {
        perform_computation_with_history(param1, param2, selected_op, history_ptr, count_ptr)
    };

    let second_op: c_int = param4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    let mut final_result = unsafe {
        perform_computation_with_history(intermediate_result, param4, second_op, history_ptr, count_ptr)
    };

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();

    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    let history_count = unsafe { *count_ptr };

    unsafe {
        printf(
            c"Computation performed at timestamp: %ld\n".as_ptr(),
            computation_time as c_long,
        );
        printf(c"Operation priority: %d\n".as_ptr(), operation_priority);
        printf(c"History entries: %d\n".as_ptr(), history_count);
        printf(c"Final result: %d\n".as_ptr(), final_result);
    }

    final_result
}
