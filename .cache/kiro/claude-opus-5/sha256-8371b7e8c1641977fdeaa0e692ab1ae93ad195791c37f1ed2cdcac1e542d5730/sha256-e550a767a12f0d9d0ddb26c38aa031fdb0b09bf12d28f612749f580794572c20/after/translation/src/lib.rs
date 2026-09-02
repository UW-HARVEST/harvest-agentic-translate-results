// Rust translation of c_src/src/lib.c
//
// Original copyright header from the C source:
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

// ---------------------------------------------------------------------------
// libc bindings.
//
// `printf` is used directly (rather than Rust's `println!`) so that the
// formatting *and* the stdio buffering behaviour are bit-for-bit the same as
// the C library's.
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn time(tloc: *mut time_t) -> time_t;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
}

/// `time_t` on the Linux/x86-64 (and every other LP64) target is a signed
/// 64-bit integer.
pub type time_t = i64;

// ---------------------------------------------------------------------------
// Types mirroring the C declarations.
// ---------------------------------------------------------------------------

// typedef enum { OP_ADD = 1, ... OP_MODULO = 5 } Operation;
//
// Enums in C are plain `int`s in this ABI and the code casts arbitrary
// (possibly out-of-range, possibly negative) integers to `Operation`, so the
// translation keeps them as `c_int`.
type Operation = c_int;

const OP_ADD: Operation = 1;
const OP_MULTIPLY: Operation = 2;
const OP_SUBTRACT: Operation = 3;
const OP_DIVIDE: Operation = 4;
const OP_MODULO: Operation = 5;

// typedef enum { STATUS_SUCCESS = 0, STATUS_ERROR = -1, STATUS_WARNING = 1 }
//     StatusCode;
type StatusCode = c_int;

const STATUS_SUCCESS: StatusCode = 0;
#[allow(dead_code)]
const STATUS_ERROR: StatusCode = -1;
#[allow(dead_code)]
const STATUS_WARNING: StatusCode = 1;

// typedef struct { int value; time_t timestamp; StatusCode status; }
//     ComputationResult;
//
// => 4 bytes + 4 bytes padding + 8 bytes + 4 bytes + 4 bytes tail padding
//    = 24 bytes, alignment 8.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: time_t,
    pub status: StatusCode,
}

// typedef int (*MathOperation)(int, int, int);
type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

/// Number of history slots the C code allocates and the fixed cap it enforces.
const HISTORY_CAPACITY: c_int = 10;

/// Signed 32-bit division/remainder with exactly C's observable behaviour on
/// this target, including the `INT_MIN / -1` overflow case.
///
/// The C code's `a / b` and `a % b` compile to a single `idiv`, which raises
/// `SIGFPE` for `INT_MIN / -1`. Rust's `/` panics there instead and
/// `wrapping_div` quietly yields `INT_MIN`, so neither matches. Emitting the
/// instruction directly keeps the trap (and every in-range result) identical.
#[cfg(target_arch = "x86_64")]
#[inline]
fn c_divrem(a: c_int, b: c_int) -> (c_int, c_int) {
    let mut quotient: c_int = a;
    let remainder: c_int;
    unsafe {
        core::arch::asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) b,
            inout("eax") quotient,
            out("edx") remainder,
            // Deliberately not `pure`/`readonly`: `idiv` may fault, and that
            // fault is observable behaviour that must not be optimised away.
            options(nostack),
        );
    }
    (quotient, remainder)
}

/// Portable fallback for non-x86-64 targets: correct for every input the C
/// program can evaluate without invoking undefined behaviour.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn c_divrem(a: c_int, b: c_int) -> (c_int, c_int) {
    (a.wrapping_div(b), a.wrapping_rem(b))
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// ```c
/// bool is_valid_operation(char op_char) {
///     char valid = op_char && (op_char >= '1' && op_char <= '5');
///     return valid;
/// }
/// ```
///
/// The intermediate `char valid` only ever holds 0 or 1, so narrowing it can
/// never discard a set bit; the result is exactly the predicate itself.
#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    let valid: c_char =
        (op_char != 0 && (op_char >= b'1' as c_char && op_char <= b'5' as c_char)) as c_char;
    valid != 0
}

/// ```c
/// int get_operation_priority(Operation op) { return op * 10; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn get_operation_priority(op: Operation) -> c_int {
    let priority = op.wrapping_mul(10);
    priority
}

/// ```c
/// int add_operation(int a, int b, int unused_param) { return a + b; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

/// ```c
/// int multiply_operation(int a, int b, int unused_param) { return a * b; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// ```c
/// int subtract_operation(int a, int b, int unused_param) { return a - b; }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// ```c
/// int divide_operation(int a, int b, int unused_param) {
///     if (b == 0) { return 0; }
///     return a / b;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_divrem(a, b).0
}

/// ```c
/// int modulo_operation(int a, int b, int unused_param) {
///     if (b == 0) { return 0; }
///     return a % b;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_divrem(a, b).1
}

/// ```c
/// MathOperation select_operation(Operation op) { switch (op) { ... } }
/// ```
///
/// Anything outside `OP_ADD..=OP_MODULO` falls through to `add_operation`,
/// exactly like the C `default:` label.
#[unsafe(no_mangle)]
pub extern "C" fn select_operation(op: Operation) -> MathOperation {
    // The exported `extern "C"` items above are ABI-compatible with
    // `MathOperation`; the transmute-free way to name them is a cast.
    match op {
        OP_ADD => add_operation as MathOperation,
        OP_MULTIPLY => multiply_operation as MathOperation,
        OP_SUBTRACT => subtract_operation as MathOperation,
        OP_DIVIDE => divide_operation as MathOperation,
        OP_MODULO => modulo_operation as MathOperation,
        _ => add_operation as MathOperation,
    }
}

/// ```c
/// time_t get_computation_timestamp() {
///     time_t current_time;
///     time(&current_time);
///     current_time = current_time >> 29;
///     return current_time;
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn get_computation_timestamp() -> time_t {
    let mut current_time: time_t = 0;
    unsafe {
        time(&mut current_time);
    }
    // Arithmetic (sign-propagating) right shift, as for a signed `time_t`.
    current_time >>= 29;
    current_time
}

/// ```c
/// ComputationResult* allocate_results(int count) {
///     return (ComputationResult*)calloc(count, sizeof(ComputationResult));
/// }
/// ```
///
/// A negative `count` is converted to `size_t` by sign extension, the same as
/// the implicit conversion in the C call, so `calloc` simply fails and returns
/// `NULL`. The C code does not check for that and neither does this.
#[unsafe(no_mangle)]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    let results =
        unsafe { calloc(count as isize as usize, core::mem::size_of::<ComputationResult>()) };
    results as *mut ComputationResult
}

/// ```c
/// int perform_computation_with_history(int a, int b, Operation op,
///                                      ComputationResult** history,
///                                      int* history_count);
/// ```
///
/// Faithful to the original, including the unchecked allocation result and the
/// hard-coded capacity of 10.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    unsafe {
        let math_func = select_operation(op);

        let result = math_func(a, b, 0);

        if (*history).is_null() {
            *history = allocate_results(HISTORY_CAPACITY);
            *history_count = 0;
        }

        if *history_count < HISTORY_CAPACITY {
            let slot = (*history).offset(*history_count as isize);
            (*slot).value = result;
            (*slot).timestamp = get_computation_timestamp();
            (*slot).status = STATUS_SUCCESS;
            *history_count = (*history_count).wrapping_add(1);
        }

        result
    }
}

// The two function-local `static` variables of `mathop`. They live for the
// whole lifetime of the loaded library and are shared by every call, so the
// history accumulates across calls just as it does in C.
static mut COMPUTATION_HISTORY: *mut ComputationResult = core::ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

/// ```c
/// int mathop(int param1, int param2, int param3, int param4);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        let mut validation_char: c_char = (param1.wrapping_rem(128)) as c_char;
        let is_valid = is_valid_operation(validation_char);

        if !is_valid {
            validation_char = b'1' as c_char;
        }
        // `validation_char` is dead from here on in the original too.
        let _ = validation_char;

        // C's `%` truncates toward zero, so a negative `param3` yields a
        // negative (out-of-range) `Operation` here. That is preserved.
        let selected_op: Operation = param3.wrapping_rem(5).wrapping_add(1);

        let operation_priority = get_operation_priority(selected_op);

        let intermediate_result = perform_computation_with_history(
            param1,
            param2,
            selected_op,
            &raw mut COMPUTATION_HISTORY,
            &raw mut HISTORY_COUNT,
        );

        let second_op: Operation = param4.wrapping_add(1).wrapping_rem(5).wrapping_add(1);
        let mut final_result = perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            &raw mut COMPUTATION_HISTORY,
            &raw mut HISTORY_COUNT,
        );

        final_result = final_result.wrapping_add(operation_priority);

        let computation_time = get_computation_timestamp();

        let time_modifier = (computation_time % 100) as c_int;
        final_result = final_result.wrapping_add(time_modifier);

        printf(
            c"Computation performed at timestamp: %ld\n".as_ptr(),
            computation_time as c_long,
        );
        printf(c"Operation priority: %d\n".as_ptr(), operation_priority);
        printf(c"History entries: %d\n".as_ptr(), HISTORY_COUNT);
        printf(c"Final result: %d\n".as_ptr(), final_result);

        final_result
    }
}
