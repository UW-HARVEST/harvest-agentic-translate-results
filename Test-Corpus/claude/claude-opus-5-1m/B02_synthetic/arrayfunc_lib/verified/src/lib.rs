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
//! Every non-static C function is reproduced with the exact same linker symbol
//! name, signature, ABI and observable behaviour (including the quirks of the
//! original code, which are intentionally *not* fixed).

#![allow(non_camel_case_types)]

use core::mem::offset_of;
use std::ffi::{c_double, c_int};

// ---------------------------------------------------------------------------
// Raw field access
// ---------------------------------------------------------------------------
//
// The C code performs completely unchecked member accesses (`arr->count`,
// `item->value`, …). Writing those as Rust place expressions (`(*arr).count`)
// makes `rustc` insert a null/alignment check whenever `-C debug-assertions` is
// on, which turns C's `SIGSEGV` on a NULL argument into a Rust `SIGABRT` with a
// panic message — an observable divergence. Going through
// `wrapping_byte_add` + `core::ptr::{read, write}` reproduces C's raw load /
// store in *every* profile: no bounds check, no null check, no alignment check.

/// Byte-offset a pointer to one of its fields. `wrapping_add` has no
/// preconditions at all, so this is well-defined even for a NULL `base`.
#[inline(always)]
fn fld<B, F>(base: *mut B, off: usize) -> *mut F {
    base.cast::<u8>().wrapping_add(off).cast::<F>()
}

#[inline(always)]
fn p_count(arr: *mut ResultArray) -> *mut c_int {
    fld(arr, offset_of!(ResultArray, count))
}

/// `&arr->data[0]` — address computation only, never a dereference (exactly
/// like the C code, which happily forms this address for a NULL `arr`).
#[inline(always)]
fn p_data(arr: *mut ResultArray) -> *mut Result {
    fld(arr, offset_of!(ResultArray, data))
}

/// `array + i`, using `wrapping_offset` so that the out-of-range indices the C
/// code never validates behave like plain pointer arithmetic.
#[inline(always)]
fn elem(base: *mut Result, i: c_int) -> *mut Result {
    base.wrapping_offset(i as isize)
}

#[inline(always)]
fn p_value(r: *mut Result) -> *mut c_int {
    fld(r, offset_of!(Result, value))
}

#[inline(always)]
fn p_scaled(r: *mut Result) -> *mut c_double {
    fld(r, offset_of!(Result, scaled))
}

#[inline(always)]
fn p_rank(r: *mut Result) -> *mut c_int {
    fld(r, offset_of!(Result, rank))
}

/// C: `typedef int (*operation_func)(int a, int b, int unused1, int unused2);`
///
/// Modelled as an `Option<...>` so that a NULL function pointer coming from C
/// is representable (calling it, like in C, is undefined behaviour).
pub type operation_func =
    Option<unsafe extern "C" fn(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int>;

/// The bare (non-nullable) shape of [`operation_func`]. Used only as the target
/// of a `transmute` at the exact point of the indirect call so that a NULL
/// pointer behaves like it does in C (a jump to address 0) instead of being
/// treated as a Rust `unreachable_unchecked`.
type operation_func_raw =
    unsafe extern "C" fn(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int;

/// C: `typedef struct { int value; double scaled; int rank; } Result;`
///
/// `#[repr(C)]` guarantees the same layout as the C struct
/// (offsets 0 / 8 / 16, size 24, align 8).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Result {
    pub value: c_int,
    pub scaled: c_double,
    pub rank: c_int,
}

/// C: `typedef struct { Result data[10]; int count; } ResultArray;`
/// (size 248, align 8)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

impl Default for ResultArray {
    fn default() -> Self {
        ResultArray {
            data: [Result::default(); 10],
            count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// C: `int add_operation(int a, int b, int unused1, int unused2)`
///
/// Signed overflow is UB in C; reproduce the wrapping behaviour that the
/// compiled C code actually exhibits on two's-complement hardware.
#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

/// C: `int multiply_operation(int a, int b, int unused1, int unused2)`
#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

/// C: `int subtract_operation(int a, int b, int unused1, int unused2)`
#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// C: `int modulo_operation(int a, int b, int unused1, int unused2)`
///
/// Keeps the original `b == 0` guard (and its `return 0`) in place.
#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    // `wrapping_rem` matches `a % b` for every input the C code can compute
    // without trapping, and avoids a Rust panic for `INT_MIN % -1` (UB in C).
    a.wrapping_rem(b)
}

// ---------------------------------------------------------------------------
// Numeric helpers
// ---------------------------------------------------------------------------

/// C: `int safe_double_to_int(double d)`
///
/// The order of the checks is preserved exactly: the saturation tests come
/// first, then the NaN test (NaN fails both comparisons, so it still reaches
/// the `d != d` branch), then the plain truncating cast.
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }
    if d != d {
        return 0;
    }
    // Here `d` is strictly inside (INT_MIN, INT_MAX), so the Rust saturating
    // cast is identical to C's truncation toward zero.
    d as c_int
}

/// C: `int compute_scaled_value(int base, double scale_factor)`
#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: c_double) -> c_int {
    let scaled = base as c_double * scale_factor;
    safe_double_to_int(scaled)
}

// ---------------------------------------------------------------------------
// Array routines
// ---------------------------------------------------------------------------

/// C: `int compare_results_in_array(ResultArray *arr, int idx1, int idx2)`
///
/// The C code compares the *addresses* `&arr->data[idx1]` and
/// `&arr->data[idx2]`. Because `data` is a contiguous array, that address
/// ordering is exactly the ordering of the indices, so comparing the indices
/// reproduces the result bit-for-bit (including for the negative indices the
/// original never validates).
///
/// # Safety
/// `arr` must be a valid pointer to a `ResultArray`, as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    let count = unsafe { core::ptr::read(p_count(arr)) };

    if idx1 >= count || idx2 >= count {
        return 0;
    }

    if idx1 < idx2 {
        -1
    } else if idx1 > idx2 {
        1
    } else {
        0
    }
}

/// C: `void init_result_array(ResultArray *arr, int values[], int count)`
///
/// Note the original clamps only the upper bound (`count < 10 ? count : 10`);
/// a negative `count` is stored verbatim and the fill loop simply does not run.
///
/// # Safety
/// `arr` must be valid, and `values` must point to at least `min(count, 10)`
/// `int`s, as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *const c_int,
    count: c_int,
) {
    let clamped = if count < 10 { count } else { 10 };
    unsafe {
        core::ptr::write(p_count(arr), clamped);
    }

    let base = p_data(arr);
    let mut i: c_int = 0;
    while i < clamped {
        let v = unsafe { core::ptr::read(values.cast_mut().wrapping_offset(i as isize)) };
        unsafe {
            core::ptr::write(
                elem(base, i),
                Result {
                    value: v,
                    scaled: v as c_double * 1.5,
                    rank: i,
                },
            );
        }
        i += 1;
    }
}

/// C: `int process_with_foreach(ResultArray *arr, operation_func op)`
///
/// The `FOREACH` macro expands to a loop whose termination test is
/// `count_iter != size` (with `size` latched once from `arr->count`); that
/// inequality — rather than a `<` bound — is reproduced faithfully.
///
/// # Safety
/// `arr` must be valid and `op` must be a callable function pointer, as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_with_foreach(arr: *mut ResultArray, op: operation_func) -> c_int {
    let mut total: c_int = 0;

    let size: c_int = unsafe { core::ptr::read(p_count(arr)) };
    // Keep the callee as an opaque address: C only dereferences `op` inside the
    // loop body, so a NULL `op` with an empty array is harmless there too.
    let op_addr: *const () = unsafe { core::mem::transmute::<operation_func, *const ()>(op) };
    let base: *mut Result = p_data(arr);

    let mut count_iter: c_int = 0;
    while count_iter != size {
        let item: *mut Result = elem(base, count_iter);

        let op: operation_func_raw =
            unsafe { core::mem::transmute::<*const (), operation_func_raw>(op_addr) };
        let result = unsafe {
            op(
                core::ptr::read(p_value(item)),
                core::ptr::read(p_rank(item)),
                0,
                0,
            )
        };
        total = total.wrapping_add(result);

        let temp = result as c_double * 0.75;
        unsafe {
            core::ptr::write(p_scaled(item), temp);
            core::ptr::write(p_value(item), safe_double_to_int(temp));
        }

        count_iter += 1;
    }

    total
}

/// C: `int compute_weighted_sum(ResultArray *arr)`
///
/// `weight` comes from the pointer difference `current - base`, i.e. the index
/// `i`, except that the `current > base` test makes index 0 use a weight of 1.
///
/// # Safety
/// `arr` must be a valid pointer to a `ResultArray`, as in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let mut sum: c_int = 0;

    let count = unsafe { core::ptr::read(p_count(arr)) };

    let mut i: c_int = 0;
    while i < count {
        let base: *mut Result = p_data(arr);
        let current: *mut Result = elem(base, i);

        let weight: c_int = if current > base {
            // `(int)(current - base)` == i
            i
        } else {
            1
        };

        let value = unsafe { core::ptr::read(p_value(current)) };
        let weighted = value as c_double * weight as c_double * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));

        i += 1;
    }

    sum
}

/// C: `int arrayfunc(int param1, int param2, int param3, int param4)`
///
/// This is the sole function declared in the public header `include/lib.h`.
#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let operations: [operation_func; 4] = [
        Some(add_operation),
        Some(multiply_operation),
        Some(subtract_operation),
        Some(modulo_operation),
    ];

    let values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        // C's `/` truncates toward zero, exactly like Rust's `/` on i32.
        param4.wrapping_div(2).wrapping_add(1),
    ];

    let mut arr = ResultArray::default();
    unsafe { init_result_array(&mut arr, values.as_ptr(), 8) };

    let mut result: c_int = 0;

    for i in 0..4usize {
        result = result.wrapping_add(unsafe { process_with_foreach(&mut arr, operations[i]) });
    }

    result = result.wrapping_add(unsafe { compute_weighted_sum(&mut arr) });

    let mut i: c_int = 0;
    while i < arr.count - 1 {
        let cmp = unsafe { compare_results_in_array(&mut arr, i, i + 1) };
        result = result.wrapping_add(cmp);
        i += 1;
    }

    let final_scale = result as c_double * 0.333;
    result = safe_double_to_int(final_scale);

    result
}
