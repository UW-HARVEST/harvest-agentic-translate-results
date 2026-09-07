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

/// Faithful reproduction of what the C compiler emits for `a / b` and `a % b`
/// on signed `int`s: a single x86-64 `cdq; idiv` pair.
///
/// This matters for exactly one input pair. `INT_MIN / -1` (and `INT_MIN % -1`)
/// is undefined behaviour in C, and on x86-64 the `idiv` instruction raises
/// `#DE`, which the kernel delivers as `SIGFPE`. The original library therefore
/// *dies* on that input, and it is reachable from the public `mathop` entry
/// point (`param1 = INT_MIN`, `param2 = -1`, `param3` selecting divide/modulo).
///
/// Rust's `wrapping_div`/`wrapping_rem` would instead quietly return `INT_MIN`
/// and `0`, and Rust's plain `/`/`%` would panic (i.e. `SIGABRT` under
/// `panic = "abort"`). Neither matches. Emitting `idiv` directly reproduces the
/// C behaviour bit-for-bit, including the `SIGFPE`.
///
/// `b == 0` is never passed in: both callers guard against it first, exactly as
/// the C does.
#[cfg(target_arch = "x86_64")]
#[inline]
fn c_signed_divrem(a: c_int, b: c_int) -> (c_int, c_int) {
    let quotient: c_int;
    let remainder: c_int;
    unsafe {
        core::arch::asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) b,
            inout("eax") a => quotient,
            out("edx") remainder,
            options(nomem, nostack),
        );
    }
    (quotient, remainder)
}

/// Portable fallback for non-x86-64 targets, where the C would emit whatever
/// that architecture's signed division does.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn c_signed_divrem(a: c_int, b: c_int) -> (c_int, c_int) {
    (a.wrapping_div(b), a.wrapping_rem(b))
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_signed_divrem(a, b).0
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_signed_divrem(a, b).1
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

    // See the `raw` module below for why these are asm loads/stores rather
    // than `*history` / `*history_count`.
    if raw::load_ptr(history).is_null() {
        raw::store_ptr(history, allocate_results(10));
        raw::store_i32(history_count, 0);
    }

    if raw::load_i32(history_count) < 10 {
        let base = raw::load_ptr(history);
        let count = raw::load_i32(history_count);
        let slot = base.offset(count as isize);
        // Field addresses are *computed*, never dereferenced through a
        // reference, so a NULL `base` faults on the store just as in C.
        raw::store_i32(&raw mut (*slot).value, result);
        raw::store_time(&raw mut (*slot).timestamp, get_computation_timestamp());
        raw::store_i32(&raw mut (*slot).status, STATUS_SUCCESS);
        raw::store_i32(history_count, count.wrapping_add(1));
    }

    result
}

/// Unchecked machine loads/stores, matching what a C compiler emits for `*p`.
///
/// WHY THIS EXISTS. The C dereferences `history` and `history_count`
/// unconditionally, with no null check — a plain `mov`. When a caller passes
/// `NULL`, the C therefore dies with `SIGSEGV` (11), and the translation must
/// die the same way.
///
/// Neither Rust spelling reproduces that under the `dev` profile:
/// with `debug-assertions = on`, both `*p` **and** `ptr::read_volatile` /
/// `ptr::write_volatile` carry an `assert_unsafe_precondition!` null check that
/// turns the fault into `panicked: null pointer dereference`. Because the panic
/// crosses an `extern "C"` boundary it becomes a non-unwinding abort, i.e.
/// `SIGABRT` (6) instead of `SIGSEGV` (11) — a divergence that the release
/// profile happens to hide. Emitting the `mov` directly has no check under any
/// profile. See ERRORS.md row 21.
#[cfg(target_arch = "x86_64")]
mod raw {
    use super::{time_t, ComputationResult};
    use core::ffi::c_int;

    #[inline]
    pub unsafe fn load_ptr(p: *mut *mut ComputationResult) -> *mut ComputationResult {
        let out: *mut ComputationResult;
        core::arch::asm!("mov {o}, qword ptr [{p}]", o = out(reg) out, p = in(reg) p,
                         options(nostack));
        out
    }

    #[inline]
    pub unsafe fn store_ptr(p: *mut *mut ComputationResult, v: *mut ComputationResult) {
        core::arch::asm!("mov qword ptr [{p}], {v}", p = in(reg) p, v = in(reg) v,
                         options(nostack));
    }

    #[inline]
    pub unsafe fn load_i32(p: *mut c_int) -> c_int {
        let out: c_int;
        core::arch::asm!("mov {o:e}, dword ptr [{p}]", o = out(reg) out, p = in(reg) p,
                         options(nostack));
        out
    }

    #[inline]
    pub unsafe fn store_i32(p: *mut c_int, v: c_int) {
        core::arch::asm!("mov dword ptr [{p}], {v:e}", p = in(reg) p, v = in(reg) v,
                         options(nostack));
    }

    #[inline]
    pub unsafe fn store_time(p: *mut time_t, v: time_t) {
        core::arch::asm!("mov qword ptr [{p}], {v}", p = in(reg) p, v = in(reg) v,
                         options(nostack));
    }
}

/// Portable fallback: `write_volatile` is the closest available primitive.
#[cfg(not(target_arch = "x86_64"))]
mod raw {
    use super::{time_t, ComputationResult};
    use core::ffi::c_int;

    #[inline]
    pub unsafe fn load_ptr(p: *mut *mut ComputationResult) -> *mut ComputationResult {
        p.read_volatile()
    }
    #[inline]
    pub unsafe fn store_ptr(p: *mut *mut ComputationResult, v: *mut ComputationResult) {
        p.write_volatile(v)
    }
    #[inline]
    pub unsafe fn load_i32(p: *mut c_int) -> c_int {
        p.read_volatile()
    }
    #[inline]
    pub unsafe fn store_i32(p: *mut c_int, v: c_int) {
        p.write_volatile(v)
    }
    #[inline]
    pub unsafe fn store_time(p: *mut time_t, v: time_t) {
        p.write_volatile(v)
    }
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
