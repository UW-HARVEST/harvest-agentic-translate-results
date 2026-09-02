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

/// `sizeof(ComputationResult)` and its field offsets, as probed on this target
/// (see the `#[repr(C)]` definition above). Used by the byte-offset accesses in
/// `perform_computation_with_history`.
const RESULT_STRIDE: isize = 24;
const OFF_VALUE: isize = 0;
const OFF_TIMESTAMP: isize = 8;
const OFF_STATUS: isize = 16;

const _: () = {
    assert!(core::mem::size_of::<ComputationResult>() == RESULT_STRIDE as usize);
    assert!(core::mem::align_of::<ComputationResult>() == 8);
    assert!(core::mem::offset_of!(ComputationResult, value) == OFF_VALUE as usize);
    assert!(core::mem::offset_of!(ComputationResult, timestamp) == OFF_TIMESTAMP as usize);
    assert!(core::mem::offset_of!(ComputationResult, status) == OFF_STATUS as usize);
};

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
// Unchecked loads and stores.
//
// The C code dereferences its `ComputationResult**` / `int*` out-parameters
// without validating them, so a NULL (or misaligned) argument makes the *load
// itself* fault with `SIGSEGV`. Rust's `*ptr` place projections carry a
// debug-assertions-only null/alignment check that panics instead, which in an
// `extern "C"` function becomes `SIGABRT` — a different observable outcome
// from the C for the same input, and one that appears only in `dev` builds.
//
// Issuing the memory access directly keeps the fault (and every non-faulting
// access) identical in every build profile.
//
// Field offsets used by the callers below come from the pinned `ComputationResult`
// ABI: `value` at +0 (4 bytes), `timestamp` at +8 (8 bytes), `status` at +16
// (4 bytes), total size 24, alignment 8.
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn raw_load64(p: *const u64) -> u64 {
    let v: u64;
    unsafe {
        core::arch::asm!(
            "mov {v}, qword ptr [{p}]",
            p = in(reg) p,
            v = out(reg) v,
            options(nostack),
        );
    }
    v
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn raw_store64(p: *mut u64, v: u64) {
    unsafe {
        core::arch::asm!(
            "mov qword ptr [{p}], {v}",
            p = in(reg) p,
            v = in(reg) v,
            options(nostack),
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn raw_load32(p: *const u32) -> u32 {
    let v: u32;
    unsafe {
        core::arch::asm!(
            "mov {v:e}, dword ptr [{p}]",
            p = in(reg) p,
            v = out(reg) v,
            options(nostack),
        );
    }
    v
}

#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn raw_store32(p: *mut u32, v: u32) {
    unsafe {
        core::arch::asm!(
            "mov dword ptr [{p}], {v:e}",
            p = in(reg) p,
            v = in(reg) v,
            options(nostack),
        );
    }
}

// Portable fallbacks. These carry Rust's debug-only pointer validity check, so
// on a non-x86-64 target a NULL out-parameter aborts rather than segfaulting in
// `dev` builds; `release` builds (the shipped artifact) fault like the C does.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
unsafe fn raw_load64(p: *const u64) -> u64 {
    unsafe { core::ptr::read(p) }
}
#[cfg(not(target_arch = "x86_64"))]
#[inline]
unsafe fn raw_store64(p: *mut u64, v: u64) {
    unsafe { core::ptr::write(p, v) }
}
#[cfg(not(target_arch = "x86_64"))]
#[inline]
unsafe fn raw_load32(p: *const u32) -> u32 {
    unsafe { core::ptr::read(p) }
}
#[cfg(not(target_arch = "x86_64"))]
#[inline]
unsafe fn raw_store32(p: *mut u32, v: u32) {
    unsafe { core::ptr::write(p, v) }
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
/// Faithful to the original, including the unchecked out-parameters, the
/// unchecked allocation result and the hard-coded capacity of 10.
///
/// Every access to `*history` / `*history_count` / `(*history)[i]` goes through
/// the `raw_*` helpers so that an invalid pointer faults exactly where and how
/// the C's load or store would, in every build profile.
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

        // `if (*history == NULL)`
        let mut hist = raw_load64(history as *const u64) as *mut ComputationResult;
        if hist.is_null() {
            // `*history = allocate_results(10); *history_count = 0;`
            hist = allocate_results(HISTORY_CAPACITY);
            raw_store64(history as *mut u64, hist as u64);
            raw_store32(history_count as *mut u32, 0);
        }

        // `if (*history_count < 10)`
        let count = raw_load32(history_count as *const u32) as c_int;
        if count < HISTORY_CAPACITY {
            // `(*history)[*history_count]` — 24-byte stride.
            let slot = (hist as *mut u8).offset(count as isize * RESULT_STRIDE);
            // `.value = result;`
            raw_store32(slot.offset(OFF_VALUE) as *mut u32, result as u32);
            // `.timestamp = get_computation_timestamp();`
            raw_store64(slot.offset(OFF_TIMESTAMP) as *mut u64, get_computation_timestamp() as u64);
            // `.status = STATUS_SUCCESS;`
            raw_store32(slot.offset(OFF_STATUS) as *mut u32, STATUS_SUCCESS as u32);
            // `(*history_count)++;`
            raw_store32(history_count as *mut u32, count.wrapping_add(1) as u32);
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
