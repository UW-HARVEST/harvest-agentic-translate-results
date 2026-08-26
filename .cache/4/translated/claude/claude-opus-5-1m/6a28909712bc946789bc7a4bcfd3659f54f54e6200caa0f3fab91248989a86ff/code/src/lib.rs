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
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_long, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings
//
// The C translation unit uses `time()`, `calloc()` and `printf()` from libc.
// They are declared (rather than re-implemented) so that behaviour -- in
// particular stdout buffering semantics and heap ownership (callers may
// `free()` the block returned by `allocate_results`) -- is bit-for-bit the
// same as the C library's.
// ---------------------------------------------------------------------------

extern "C" {
    fn time(tloc: *mut time_t) -> time_t;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
}

/// `time_t` on the target ABI (x86-64 / aarch64 Linux: 64-bit signed).
pub type time_t = i64;

// ---------------------------------------------------------------------------
// typedef enum { OP_ADD = 1, ... } Operation;
// ---------------------------------------------------------------------------

/// `Operation` is an `int`-sized C enum, so it is modelled as `c_int`.
pub type Operation = c_int;

pub const OP_ADD: Operation = 1;
pub const OP_MULTIPLY: Operation = 2;
pub const OP_SUBTRACT: Operation = 3;
pub const OP_DIVIDE: Operation = 4;
pub const OP_MODULO: Operation = 5;

// ---------------------------------------------------------------------------
// typedef enum { STATUS_SUCCESS = 0, STATUS_ERROR = -1, STATUS_WARNING = 1 }
//     StatusCode;
// ---------------------------------------------------------------------------

/// `StatusCode` is an `int`-sized C enum (it has a negative enumerator), so it
/// is modelled as `c_int`.
pub type StatusCode = c_int;

pub const STATUS_SUCCESS: StatusCode = 0;
pub const STATUS_ERROR: StatusCode = -1;
pub const STATUS_WARNING: StatusCode = 1;

// ---------------------------------------------------------------------------
// typedef struct { int value; time_t timestamp; StatusCode status; }
//     ComputationResult;
//
// Layout on the reference ABI: size 24, align 8,
// offsets value = 0, timestamp = 8, status = 16.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: time_t,
    pub status: StatusCode,
}

// ---------------------------------------------------------------------------
// typedef int (*MathOperation)(int, int, int);
// ---------------------------------------------------------------------------

pub type MathOperation = extern "C" fn(c_int, c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// bool is_valid_operation(char op_char)
// ---------------------------------------------------------------------------

/// ```c
/// bool is_valid_operation(char op_char) {
///     char valid = op_char && (op_char >= '1' && op_char <= '5');
///     return valid;
/// }
/// ```
///
/// `char` is signed on the reference ABI, hence `c_char`. The intermediate
/// `char valid` only ever holds 0 or 1, and the `_Bool` conversion is
/// `valid != 0`; both steps are reproduced literally.
#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    const ONE: c_char = b'1' as c_char;
    const FIVE: c_char = b'5' as c_char;

    let condition = (op_char != 0) && (op_char >= ONE && op_char <= FIVE);
    let valid: c_char = if condition { 1 } else { 0 };
    valid != 0
}

// ---------------------------------------------------------------------------
// int get_operation_priority(Operation op)
// ---------------------------------------------------------------------------

/// ```c
/// int get_operation_priority(Operation op) {
///     int priority = op * 10;
///     return priority;
/// }
/// ```
///
/// `wrapping_mul` matches the two's-complement result GCC/Clang produce for
/// signed overflow; no bug is "fixed" here.
#[unsafe(no_mangle)]
pub extern "C" fn get_operation_priority(op: Operation) -> c_int {
    let priority = op.wrapping_mul(10);
    priority
}

// ---------------------------------------------------------------------------
// The five MathOperation implementations.
//
// `unused_param` is present in the C signature and deliberately ignored, so
// the ABI is preserved exactly.
// ---------------------------------------------------------------------------

/// `int add_operation(int a, int b, int unused_param) { return a + b; }`
#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

/// `int multiply_operation(int a, int b, int unused_param) { return a * b; }`
#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// `int subtract_operation(int a, int b, int unused_param) { return a - b; }`
#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// ```c
/// int divide_operation(int a, int b, int unused_param) {
///     if (b == 0) {
///         return 0;
///     }
///     return a / b;
/// }
/// ```
///
/// The divide-by-zero guard is kept in exactly the same position, and
/// `wrapping_div` gives C's truncating-toward-zero quotient (e.g. `-7 / 2 ==
/// -3`).
///
/// One documented divergence: `divide_operation(INT_MIN, -1, _)`. That is
/// signed-overflow *undefined behaviour* in C -- the reference library executes
/// `idiv` and dies from SIGFPE, so it has no defined output to be identical to.
/// `wrapping_div` returns `INT_MIN` here instead of panicking or killing the
/// process. Every input with defined C behaviour matches bit-for-bit.
#[unsafe(no_mangle)]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

/// ```c
/// int modulo_operation(int a, int b, int unused_param) {
///     if (b == 0) {
///         return 0;
///     }
///     return a % b;
/// }
/// ```
///
/// `wrapping_rem` reproduces C's truncating remainder, which keeps the sign of
/// the dividend (e.g. `-3 % 5 == -3`). As with `divide_operation`, the
/// `INT_MIN % -1` case is UB in C (SIGFPE on the reference build) and yields 0
/// here rather than aborting.
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

/// ```c
/// MathOperation select_operation(Operation op) {
///     switch (op) {
///         case OP_ADD:      return add_operation;
///         case OP_MULTIPLY: return multiply_operation;
///         case OP_SUBTRACT: return subtract_operation;
///         case OP_DIVIDE:   return divide_operation;
///         case OP_MODULO:   return modulo_operation;
///         default:          return add_operation;
///     }
/// }
/// ```
///
/// The returned pointers are the addresses of the exported symbols, so callers
/// can compare them against `add_operation` &c. just as with the C library.
#[unsafe(no_mangle)]
pub extern "C" fn select_operation(op: Operation) -> MathOperation {
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

/// ```c
/// time_t get_computation_timestamp() {
///     time_t current_time;
///     time(&current_time);
///     current_time = current_time >> 29;
///     return current_time;
/// }
/// ```
///
/// The `time()` return value is discarded in the C source exactly as here; the
/// value comes from the out-parameter. `>>` on a signed `time_t` is an
/// arithmetic shift, which `i64 >> 29` also performs.
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

/// ```c
/// ComputationResult* allocate_results(int count) {
///     ComputationResult* results =
///         (ComputationResult*)calloc(count, sizeof(ComputationResult));
///     return results;
/// }
/// ```
///
/// `calloc` from libc is used so the block remains `free()`-able by callers.
/// `count as usize` sign-extends negative counts precisely like C's
/// `int` -> `size_t` conversion (so `calloc` sees a huge request and returns
/// NULL), and no NULL check is added because the C code has none.
#[unsafe(no_mangle)]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    let results =
        unsafe { calloc(count as usize, size_of::<ComputationResult>()) } as *mut ComputationResult;
    results
}

// ---------------------------------------------------------------------------
// int perform_computation_with_history(int a, int b, Operation op,
//                                      ComputationResult** history,
//                                      int* history_count)
// ---------------------------------------------------------------------------

/// ```c
/// int perform_computation_with_history(int a, int b, Operation op,
///                                      ComputationResult** history,
///                                      int* history_count) {
///     MathOperation math_func = select_operation(op);
///     int result = math_func(a, b, 0);
///     if (*history == NULL) {
///         *history = allocate_results(10);
///         *history_count = 0;
///     }
///     if (*history_count < 10) {
///         (*history)[*history_count].value = result;
///         (*history)[*history_count].timestamp = get_computation_timestamp();
///         (*history)[*history_count].status = STATUS_SUCCESS;
///         (*history_count)++;
///     }
///     return result;
/// }
/// ```
///
/// The order of the two `if`s, the field assignment order and the timestamp
/// call site are all preserved. Note that the C code dereferences `*history`
/// without re-checking it after `allocate_results`, so a failed allocation
/// faults; that behaviour is reproduced rather than fixed.
///
/// # Safety
/// `history` and `history_count` must be valid, writable pointers, and
/// `*history` must be either NULL or an array of at least 10
/// `ComputationResult`s.
#[unsafe(no_mangle)]
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
        let index = *history_count;
        let entry = (*history).offset(index as isize);
        (*entry).value = result;
        (*entry).timestamp = get_computation_timestamp();
        (*entry).status = STATUS_SUCCESS;
        *history_count = index.wrapping_add(1);
    }

    result
}

// ---------------------------------------------------------------------------
// int mathop(int param1, int param2, int param3, int param4)
//
// The `static` locals below correspond to:
//     static ComputationResult* computation_history = NULL;
//     static int history_count = 0;
// They are process-global and, exactly like the C original, not thread-safe.
// ---------------------------------------------------------------------------

static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

/// The public entry point declared in `include/lib.h`:
/// `int mathop(int a, int b, int c, int d);`
///
/// Every computation step and all four `printf` calls (same format strings,
/// same order) are reproduced verbatim, and the real libc `printf` is used so
/// stdout buffering/interleaving is identical to the C library's.
///
/// `%` on negative operands truncates toward zero in both C and Rust, so
/// `param3 % 5` can be negative and `selected_op` can therefore fall outside
/// `OP_ADD..=OP_MODULO`; `select_operation` then takes its `default` branch.
/// This quirk is intentionally preserved.
#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    // `static ComputationResult* computation_history` / `static int history_count`
    let computation_history: *mut *mut ComputationResult = &raw mut COMPUTATION_HISTORY;
    let history_count: *mut c_int = &raw mut HISTORY_COUNT;

    // char validation_char = (char)(param1 % 128);
    let mut validation_char: c_char = param1.wrapping_rem(128) as c_char;

    // bool is_valid = is_valid_operation(validation_char);
    let is_valid = is_valid_operation(validation_char);

    // if (!is_valid) { validation_char = '1'; }
    // (dead store in the original -- kept for fidelity)
    if !is_valid {
        validation_char = b'1' as c_char;
    }
    let _ = validation_char;

    // Operation selected_op = (Operation)((param3 % 5) + 1);
    let selected_op: Operation = param3.wrapping_rem(5).wrapping_add(1);

    // int operation_priority = get_operation_priority(selected_op);
    let operation_priority = get_operation_priority(selected_op);

    // int intermediate_result = perform_computation_with_history(...);
    let intermediate_result = unsafe {
        perform_computation_with_history(
            param1,
            param2,
            selected_op,
            computation_history,
            history_count,
        )
    };

    // Operation second_op = (Operation)(((param4 + 1) % 5) + 1);
    let second_op: Operation = param4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);

    // int final_result = perform_computation_with_history(...);
    let mut final_result = unsafe {
        perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            computation_history,
            history_count,
        )
    };

    // final_result += operation_priority;
    final_result = final_result.wrapping_add(operation_priority);

    // time_t computation_time = get_computation_timestamp();
    let computation_time = get_computation_timestamp();

    // int time_modifier = (int)(computation_time % 100);
    let time_modifier = (computation_time % 100) as c_int;

    // final_result += time_modifier;
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
