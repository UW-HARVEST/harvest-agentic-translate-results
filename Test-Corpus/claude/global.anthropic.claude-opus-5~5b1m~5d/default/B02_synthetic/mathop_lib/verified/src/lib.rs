// Rust translation of c_src/src/lib.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// C library bindings (libc). Using the platform C library keeps stdio
// buffering / allocation behaviour byte-for-byte identical with the original.
// ---------------------------------------------------------------------------

/// `time_t` on the target platforms this library is built for (LP64 Linux).
pub type time_t = c_long;

extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn time(tloc: *mut time_t) -> time_t;
    fn printf(format: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Types mirroring the C declarations
// ---------------------------------------------------------------------------

// typedef enum { OP_ADD = 1, ... } Operation;  -> passed as a plain C int.
pub const OP_ADD: c_int = 1;
pub const OP_MULTIPLY: c_int = 2;
pub const OP_SUBTRACT: c_int = 3;
pub const OP_DIVIDE: c_int = 4;
pub const OP_MODULO: c_int = 5;

// typedef enum { STATUS_SUCCESS = 0, STATUS_ERROR = -1, STATUS_WARNING = 1 } StatusCode;
pub const STATUS_SUCCESS: c_int = 0;
pub const _STATUS_ERROR: c_int = -1;
pub const _STATUS_WARNING: c_int = 1;

// typedef struct { int value; time_t timestamp; StatusCode status; } ComputationResult;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: time_t,
    pub status: c_int,
}

// typedef int (*MathOperation)(int, int, int);
pub type MathOperation = extern "C" fn(c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// bool is_valid_operation(char op_char)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    // char valid = op_char && (op_char >= '1' && op_char <= '5');
    let valid: c_char = (op_char != 0 && (op_char >= b'1' as c_char && op_char <= b'5' as c_char))
        as c_char;
    valid != 0
}

// ---------------------------------------------------------------------------
// int get_operation_priority(Operation op)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn get_operation_priority(op: c_int) -> c_int {
    op.wrapping_mul(10)
}

// ---------------------------------------------------------------------------
// The individual math operations
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// MathOperation select_operation(Operation op)
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// time_t get_computation_timestamp(void)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    unsafe {
        time(&mut current_time);
    }
    current_time >>= 29;
    current_time
}

// ---------------------------------------------------------------------------
// ComputationResult* allocate_results(int count)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    // calloc(count, sizeof(ComputationResult)) -- `count` is converted to
    // size_t exactly as C would (sign extension for negative values).
    let nmemb = count as isize as usize;
    let results = unsafe { calloc(nmemb, core::mem::size_of::<ComputationResult>()) };
    results as *mut ComputationResult
}

// ---------------------------------------------------------------------------
// int perform_computation_with_history(int a, int b, Operation op,
//                                      ComputationResult** history,
//                                      int* history_count)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func: MathOperation = select_operation(op);

    let result = math_func(a, b, 0);

    if (*history).is_null() {
        *history = allocate_results(10);
        *history_count = 0;
    }

    if *history_count < 10 {
        let slot = (*history).offset(*history_count as isize);
        (*slot).value = result;
        (*slot).timestamp = get_computation_timestamp();
        (*slot).status = STATUS_SUCCESS;
        *history_count = (*history_count).wrapping_add(1);
    }

    result
}

// ---------------------------------------------------------------------------
// int mathop(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------

// The two function-local `static` variables of `mathop`.
static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let computation_history: *mut *mut ComputationResult = &raw mut COMPUTATION_HISTORY;
    let history_count: *mut c_int = &raw mut HISTORY_COUNT;

    let mut validation_char: c_char = (param1.wrapping_rem(128)) as c_char;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as c_char;
    }
    let _ = validation_char;

    let selected_op: c_int = param3.wrapping_rem(5).wrapping_add(1);

    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = unsafe {
        perform_computation_with_history(
            param1,
            param2,
            selected_op,
            computation_history,
            history_count,
        )
    };

    let second_op: c_int = param4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
    let mut final_result = unsafe {
        perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            computation_history,
            history_count,
        )
    };

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();

    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    unsafe {
        printf(
            b"Computation performed at timestamp: %ld\n\0".as_ptr() as *const c_char,
            computation_time as c_long,
        );
        printf(
            b"Operation priority: %d\n\0".as_ptr() as *const c_char,
            operation_priority,
        );
        printf(
            b"History entries: %d\n\0".as_ptr() as *const c_char,
            *history_count,
        );
        printf(
            b"Final result: %d\n\0".as_ptr() as *const c_char,
            final_result,
        );
    }

    final_result
}
