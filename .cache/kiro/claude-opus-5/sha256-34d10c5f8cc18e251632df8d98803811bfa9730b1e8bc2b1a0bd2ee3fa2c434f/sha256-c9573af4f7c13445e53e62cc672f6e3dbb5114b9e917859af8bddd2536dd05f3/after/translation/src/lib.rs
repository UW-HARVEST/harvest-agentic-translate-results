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

use std::ffi::{c_double, c_int};

/// `typedef int (*operation_func)(int a, int b, int unused1, int unused2);`
pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// ```c
/// typedef struct {
///     int value;
///     double scaled;
///     int rank;
/// } Result;
/// ```
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
#[repr(C)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    // C signed overflow is UB; gcc/clang produce two's-complement wraparound.
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

/// Signed remainder with C's exact machine semantics.
///
/// On x86-64 the compiler emits `idiv`, which raises `#DE` (SIGFPE) for
/// `INT_MIN % -1`. Rust's `%` would instead panic there, so the instruction is
/// emitted directly to keep the observable behaviour identical to the C.
#[cfg(target_arch = "x86_64")]
#[inline]
fn c_rem_i32(a: c_int, b: c_int) -> c_int {
    let rem: c_int;
    unsafe {
        std::arch::asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) b,
            inout("eax") a => _,
            out("edx") rem,
            options(nomem, nostack),
        );
    }
    rem
}

#[cfg(not(target_arch = "x86_64"))]
#[inline]
fn c_rem_i32(a: c_int, b: c_int) -> c_int {
    a.wrapping_rem(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    c_rem_i32(a, b)
}

// ---------------------------------------------------------------------------
// Numeric helpers
// ---------------------------------------------------------------------------

/// Exact order of checks preserved: upper bound, lower bound, then NaN.
/// (NaN fails both relational tests, so it reaches the `d != d` case.)
#[unsafe(no_mangle)]
// `d != d` is kept verbatim from `lib.c:82` rather than rewritten to
// `d.is_nan()`; the two are identical in codegen and the literal form preserves
// the line-by-line correspondence with the C source.
#[allow(clippy::eq_op)]
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
    // Value is strictly inside the int range here, so truncation matches C.
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: c_double) -> c_int {
    let scaled = base as c_double * scale_factor;
    safe_double_to_int(scaled)
}

// ---------------------------------------------------------------------------
// ResultArray operations
// ---------------------------------------------------------------------------

/// Compares the *addresses* of two elements, exactly as the C does. No lower
/// bound check on the indices is performed (faithful to the original).
#[unsafe(no_mangle)]
/// # Safety
/// `arr` must be a valid `ResultArray*`. As in the C, the indices are only
/// checked against `arr->count` from above — a negative index or a `count`
/// larger than the real array forms an out-of-bounds address (never
/// dereferenced), exactly as `&arr->data[idx]` does in C.
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    let count = (*arr).count;
    if idx1 >= count || idx2 >= count {
        return 0;
    }

    let base = (&raw mut (*arr).data) as *mut Result;
    let ptr1 = base.offset(idx1 as isize);
    let ptr2 = base.offset(idx2 as isize);

    if ptr1 < ptr2 {
        -1
    } else if ptr1 > ptr2 {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
/// # Safety
/// `arr` must be a valid `ResultArray*`. `values` must point to at least
/// `min(count, 10)` `int`s; when that is zero, `values` is never read (so NULL
/// is accepted), matching the C.
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    (*arr).count = if count < 10 { count } else { 10 };

    let base = (&raw mut (*arr).data) as *mut Result;
    let mut i: c_int = 0;
    while i < (*arr).count {
        let v = *values.offset(i as isize);
        *base.offset(i as isize) = Result {
            value: v,
            scaled: v as c_double * 1.5,
            rank: i,
        };
        i += 1;
    }
}

/// Translation of the `FOREACH` macro loop: walks `arr->data[0 .. arr->count)`.
#[unsafe(no_mangle)]
/// # Safety
/// `arr` must be a valid `ResultArray*` and `op` a callable function pointer.
/// A negative `arr->count` makes the loop run off the end of the array, exactly
/// as the `count_iter != size` guard in the C `FOREACH` macro does.
pub unsafe extern "C" fn process_with_foreach(arr: *mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;

    let base = (&raw mut (*arr).data) as *mut Result;
    let size = (*arr).count;

    let mut count_iter: c_int = 0;
    while count_iter != size {
        let item = base.offset(count_iter as isize);

        let result = op((*item).value, (*item).rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = result as c_double * 0.75;
        (*item).scaled = temp;
        (*item).value = safe_double_to_int(temp);

        count_iter += 1;
    }

    total
}

#[unsafe(no_mangle)]
/// # Safety
/// `arr` must be a valid `ResultArray*`; elements `0..arr->count` are read.
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let mut sum: c_int = 0;

    let base_ptr = (&raw mut (*arr).data) as *mut Result;
    let count = (*arr).count;

    let mut i: c_int = 0;
    while i < count {
        let current = base_ptr.offset(i as isize);
        let base = base_ptr;

        // `current - base` is the element index; 1 is used for the first slot.
        let weight: c_int = if current > base {
            current.offset_from(base) as c_int
        } else {
            1
        };

        let weighted = (*current).value as c_double * weight as c_double * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));

        i += 1;
    }

    sum
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let operations: [OperationFunc; 4] = [
        add_operation,
        multiply_operation,
        subtract_operation,
        modulo_operation,
    ];

    let mut values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        (param4 / 2).wrapping_add(1),
    ];

    // `ResultArray arr = {.count = 0};` zero-initializes the whole aggregate.
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
    }

    let mut result: c_int = 0;

    for i in 0..4 {
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
