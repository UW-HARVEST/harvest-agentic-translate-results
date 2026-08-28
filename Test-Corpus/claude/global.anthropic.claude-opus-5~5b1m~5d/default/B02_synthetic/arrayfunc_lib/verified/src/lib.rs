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

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Rust translation of `c_src/src/lib.c`.
//!
//! The C translation unit contains no `static` functions, so every function it
//! defines is part of the shared library's public ABI:
//!
//! * `add_operation`
//! * `multiply_operation`
//! * `subtract_operation`
//! * `modulo_operation`
//! * `safe_double_to_int`
//! * `compute_scaled_value`
//! * `compare_results_in_array`
//! * `init_result_array`
//! * `process_with_foreach`
//! * `compute_weighted_sum`
//! * `arrayfunc`
//!
//! All of them are re-exported here with identical linker names, signatures and
//! (bug-for-bug) behaviour.

#![allow(non_camel_case_types)]

use std::os::raw::{c_double, c_int};

/// `typedef int (*operation_func)(int a, int b, int unused1, int unused2);`
///
/// Modelled as an `Option<extern "C" fn ...>` so that a NULL function pointer
/// coming from C can be represented; the ABI is identical to a bare C function
/// pointer thanks to the null-pointer optimization.
pub type operation_func =
    Option<unsafe extern "C" fn(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int>;

/// ```c
/// typedef struct {
///     int value;
///     double scaled;
///     int rank;
/// } Result;
/// ```
///
/// sizeof == 24, offsets: value @ 0, scaled @ 8, rank @ 16.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Result {
    pub value: c_int,
    pub scaled: c_double,
    pub rank: c_int,
}

/// ```c
/// typedef struct {
///     Result data[10];
///     int count;
/// } ResultArray;
/// ```
///
/// sizeof == 248, offsets: data @ 0, count @ 240.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Arithmetic operations
// ---------------------------------------------------------------------------
//
// The C code performs plain `int` arithmetic; signed overflow is UB in C but in
// practice gcc/clang emit two's-complement wrapping instructions.  `wrapping_*`
// reproduces exactly what the compiled C does (and never panics).

/// `int add_operation(int a, int b, int unused1, int unused2)`
#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

/// `int multiply_operation(int a, int b, int unused1, int unused2)`
#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// `int subtract_operation(int a, int b, int unused1, int unused2)`
#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// The C `%` operator, reproduced *exactly* as the compiler emits it.
///
/// This matters for `INT_MIN % -1`: C evaluates the remainder with a single
/// `idivl`, whose implicit quotient (`2147483648`) does not fit in `eax`, so the
/// CPU raises `#DE` and the process dies with **SIGFPE**.  Neither of Rust's
/// native operators reproduces that: `a % b` panics (SIGABRT under
/// `panic = "abort"`) and `a.wrapping_rem(b)` quietly returns `0`.  Emitting the
/// instruction directly is the only faithful translation — verified against the
/// compiled C, which exits with signal 8 for this input.
#[cfg(target_arch = "x86_64")]
#[inline]
fn c_int_rem(a: c_int, b: c_int) -> c_int {
    let rem: c_int;
    unsafe {
        core::arch::asm!(
            "cdq",            // sign-extend eax into edx:eax
            "idiv {b:e}",     // quotient -> eax, remainder -> edx (may raise #DE)
            b = in(reg) b,
            inout("eax") a => _,
            out("edx") rem,
            // Deliberately *not* `pure`: the trap is an observable side effect
            // and must never be optimised away.
            options(nomem, nostack),
        );
    }
    rem
}

/// Portable fallback for non-x86-64 hosts, where the C compiler would not emit
/// `idiv` either.
#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn c_int_rem(a: c_int, b: c_int) -> c_int {
    a.wrapping_rem(b)
}

/// `int modulo_operation(int a, int b, int unused1, int unused2)`
///
/// Keeps the `b == 0` guard in front, exactly as the C does; every other input
/// goes through the same `idiv` the C compiler emits (see [`c_int_rem`]).
#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_int_rem(a, b)
}

// ---------------------------------------------------------------------------
// Double -> int conversion
// ---------------------------------------------------------------------------

/// ```c
/// int safe_double_to_int(double d) {
///     if (d >= (double)INT32_MAX) return INT32_MAX;
///     if (d <= (double)INT32_MIN) return INT32_MIN;
///     if (d != d) return 0;
///     return (int)d;
/// }
/// ```
///
/// The order of the checks is preserved verbatim: NaN fails both relational
/// tests (they are always false) and is therefore caught by the `d != d` test.
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d >= i32::MAX as c_double {
        return i32::MAX;
    }
    if d <= i32::MIN as c_double {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    // Here `d` is strictly inside (INT32_MIN, INT32_MAX), so the saturating
    // Rust cast performs exactly the same truncation-toward-zero as C.
    d as c_int
}

/// `int compute_scaled_value(int base, double scale_factor)`
#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: c_double) -> c_int {
    let scaled = base as c_double * scale_factor;
    safe_double_to_int(scaled)
}

// ---------------------------------------------------------------------------
// ResultArray helpers
// ---------------------------------------------------------------------------

/// ```c
/// int compare_results_in_array(ResultArray *arr, int idx1, int idx2) {
///     if (idx1 >= arr->count || idx2 >= arr->count) return 0;
///     Result *ptr1 = &arr->data[idx1];
///     Result *ptr2 = &arr->data[idx2];
///     if (ptr1 < ptr2) return -1;
///     else if (ptr1 > ptr2) return 1;
///     return 0;
/// }
/// ```
///
/// Note the C code only bounds-checks the *upper* limit (negative indices are
/// accepted, as in the original); the comparison is done on the *addresses* of
/// the elements, which is monotonic in the index.  This bug-for-bug behaviour
/// is retained by comparing the computed addresses.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    if idx1 >= (*arr).count || idx2 >= (*arr).count {
        return 0;
    }

    let base: *mut Result = arr.cast::<Result>();
    // `wrapping_offset` keeps out-of-range indices from being insta-UB while
    // producing the very same address arithmetic the C compiler emits.
    let ptr1: *mut Result = base.wrapping_offset(idx1 as isize);
    let ptr2: *mut Result = base.wrapping_offset(idx2 as isize);

    if ptr1 < ptr2 {
        -1
    } else if ptr1 > ptr2 {
        1
    } else {
        0
    }
}

/// ```c
/// void init_result_array(ResultArray *arr, int values[], int count) {
///     arr->count = count < 10 ? count : 10;
///     for (int i = 0; i < arr->count; i++) {
///         arr->data[i] = (Result){ .value = values[i],
///                                  .scaled = (double)values[i] * 1.5,
///                                  .rank = i };
///     }
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    (*arr).count = if count < 10 { count } else { 10 };

    let base: *mut Result = arr.cast::<Result>();
    let mut i: c_int = 0;
    // The C loop re-reads `arr->count` on every iteration; nothing in the body
    // modifies it, so a plain comparison against the stored value is exact.
    while i < (*arr).count {
        let v: c_int = *values.wrapping_offset(i as isize);
        *base.wrapping_offset(i as isize) = Result {
            value: v,
            scaled: v as c_double * 1.5,
            rank: i,
        };
        i = i.wrapping_add(1);
    }
}

/// ```c
/// int process_with_foreach(ResultArray *arr, operation_func op) {
///     int total = 0;
///     Result *item;
///     FOREACH(item, arr->data, arr->count) {
///         int result = op(item->value, item->rank, 0, 0);
///         total += result;
///         double temp = (double)result * 0.75;
///         item->scaled = temp;
///         item->value = safe_double_to_int(temp);
///     }
///     return total;
/// }
/// ```
///
/// The `FOREACH` macro expands to
///
/// ```c
/// for (int keep = 1, count_iter = 0, size = (count);
///      keep && count_iter != size;
///      keep = !keep, count_iter++)
///   for (item = (array) + count_iter; keep; keep = !keep)
/// ```
///
/// i.e. `size` is evaluated exactly once, the loop terminates on `!=` (not
/// `<`), and `item` walks `arr->data` element by element.  The `keep` flag only
/// exists so that the inner `for` runs the body once; it does not affect the
/// visible semantics.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_with_foreach(arr: *mut ResultArray, op: operation_func) -> c_int {
    let mut total: c_int = 0;

    // `data` is at offset 0 of `ResultArray`, so casting the incoming pointer
    // keeps the provenance of the whole struct instead of narrowing it to the
    // `data` field.
    let base: *mut Result = arr.cast::<Result>();
    let size: c_int = (*arr).count; // evaluated once, as in the macro

    let mut count_iter: c_int = 0;
    while count_iter != size {
        let item: *mut Result = base.wrapping_offset(count_iter as isize);

        // The call is resolved *inside* the loop, exactly like the C: when the
        // loop body never runs (`count == 0`) the C never dereferences `op`, so
        // a NULL `op` must be harmless here too.
        //
        // The pointer is `transmute`d rather than `unwrap_unchecked()`ed: for a
        // NULL `op` the C jumps to address 0 and dies with SIGSEGV, whereas
        // `unwrap_unchecked` would trip Rust's `unreachable_unchecked`
        // instrumentation and abort with SIGABRT instead.  Transmuting calls
        // through the raw pointer value exactly as C does.
        let f: unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int =
            core::mem::transmute(op);
        let result: c_int = f((*item).value, (*item).rank, 0, 0);
        total = total.wrapping_add(result);

        let temp: c_double = result as c_double * 0.75;
        (*item).scaled = temp;
        (*item).value = safe_double_to_int(temp);

        count_iter = count_iter.wrapping_add(1);
    }

    total
}

/// ```c
/// int compute_weighted_sum(ResultArray *arr) {
///     int sum = 0;
///     for (int i = 0; i < arr->count; i++) {
///         Result *current = &arr->data[i];
///         Result *base = &arr->data[0];
///         int weight = (current > base) ? (int)(current - base) : 1;
///         double weighted = (double)current->value * (double)weight * 0.8;
///         sum += safe_double_to_int(weighted);
///     }
///     return sum;
/// }
/// ```
///
/// `current - base` is a pointer difference in *elements*, i.e. `i`; for `i == 0`
/// the pointers are equal so the weight is 1 (not 0) — preserved as-is.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let mut sum: c_int = 0;

    let mut i: c_int = 0;
    while i < (*arr).count {
        let base: *mut Result = arr.cast::<Result>();
        let current: *mut Result = base.wrapping_offset(i as isize);

        // `current > base` holds exactly when `i > 0`, and the C pointer
        // difference `(int)(current - base)` is exactly `i` (the difference is
        // counted in elements).  Computing it arithmetically instead of with
        // `offset_from` avoids Rust's "both pointers must be in bounds of the
        // same object" precondition while producing bit-identical results even
        // when a caller hand-sets `count` past the 10-element capacity.
        let weight: c_int = if i > 0 { i } else { 1 };
        let _ = current;

        // Evaluation order matters for bit-exact floating point:
        // ((double)value * (double)weight) * 0.8
        let weighted: c_double = (*current).value as c_double * weight as c_double * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));

        i = i.wrapping_add(1);
    }

    sum
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// ```c
/// int arrayfunc(int param1, int param2, int param3, int param4);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let operations: [operation_func; 4] = [
        Some(add_operation_shim),
        Some(multiply_operation_shim),
        Some(subtract_operation_shim),
        Some(modulo_operation_shim),
    ];

    let mut values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        // `param4 / 2` truncates toward zero (the divisor is a constant 2, so
        // the INT_MIN / -1 trap case cannot occur).
        (param4 / 2).wrapping_add(1),
    ];

    // `ResultArray arr = {.count = 0};` zero-initialises every other member.
    let mut arr = ResultArray {
        data: [Result {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
        count: 0,
    };

    unsafe {
        init_result_array(&mut arr, values.as_mut_ptr(), 8);

        let mut result: c_int = 0;

        let mut i: c_int = 0;
        while i < 4 {
            result = result.wrapping_add(process_with_foreach(&mut arr, operations[i as usize]));
            i += 1;
        }

        result = result.wrapping_add(compute_weighted_sum(&mut arr));

        let mut i: c_int = 0;
        while i < arr.count.wrapping_sub(1) {
            let cmp = compare_results_in_array(&mut arr, i, i.wrapping_add(1));
            result = result.wrapping_add(cmp);
            i = i.wrapping_add(1);
        }

        let final_scale: c_double = result as c_double * 0.333;
        result = safe_double_to_int(final_scale);

        result
    }
}

// ---------------------------------------------------------------------------
// `unsafe extern "C"` shims
//
// `operation_func` is an `unsafe extern "C" fn` pointer type (so that it can
// model an arbitrary pointer received from C), while the four exported
// operations are safe `extern "C" fn`s.  These zero-cost shims bridge the two;
// they are `#[inline]` and never exported, so the ABI is unaffected.
// ---------------------------------------------------------------------------

#[inline(always)]
unsafe extern "C" fn add_operation_shim(a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
    add_operation(a, b, u1, u2)
}

#[inline(always)]
unsafe extern "C" fn multiply_operation_shim(a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
    multiply_operation(a, b, u1, u2)
}

#[inline(always)]
unsafe extern "C" fn subtract_operation_shim(a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
    subtract_operation(a, b, u1, u2)
}

#[inline(always)]
unsafe extern "C" fn modulo_operation_shim(a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
    modulo_operation(a, b, u1, u2)
}
