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
//! Behaviour is reproduced exactly, including the quirks of the original:
//!   * `compare_results_in_array` compares the *addresses* of two array slots,
//!     so it effectively just compares the two indices (and only bounds-checks
//!     the upper end, never negatives).
//!   * `compute_weighted_sum` derives the weight from a pointer difference,
//!     which equals the element index, with index 0 special-cased to 1.
//!   * `int` arithmetic wraps (matching what the C compiler emits in practice
//!     for signed overflow).

use std::ffi::c_int;

/// `typedef int (*operation_func)(int a, int b, int unused1, int unused2);`
pub type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

/// Nullable form used for the FFI boundary, where the caller may pass NULL.
pub type OperationFuncOpt = Option<OperationFunc>;

const MAX_RESULTS: usize = 10;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Result_ {
    pub value: c_int,
    pub scaled: f64,
    pub rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ResultArray {
    pub data: [Result_; MAX_RESULTS],
    pub count: c_int,
}

impl Default for ResultArray {
    fn default() -> Self {
        // Mirrors `ResultArray arr = {.count = 0};` which zero-initialises
        // every member, `data` included.
        Self {
            data: [Result_::default(); MAX_RESULTS],
            count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
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

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

// ---------------------------------------------------------------------------
// Scalar helpers
// ---------------------------------------------------------------------------

/// Order of the checks matters and is preserved: the two range comparisons run
/// before the NaN test. NaN compares false against everything, so it falls
/// through to the `d != d` test and yields 0.
fn safe_double_to_int_impl(d: f64) -> c_int {
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    // The guards above leave `d` strictly inside the i32 range, so this is a
    // plain truncation toward zero, exactly like the C cast.
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    safe_double_to_int_impl(d)
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = (base as f64) * scale_factor;
    safe_double_to_int_impl(scaled)
}

// ---------------------------------------------------------------------------
// Array helpers
// ---------------------------------------------------------------------------

/// The C version takes the addresses of `data[idx1]` / `data[idx2]` and
/// compares the pointers. Since the slots are contiguous and laid out in index
/// order, that is exactly a comparison of the indices themselves.
fn compare_results_in_array_impl(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 >= arr.count || idx2 >= arr.count {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    compare_results_in_array_impl(unsafe { &*arr }, idx1, idx2)
}

fn init_result_array_impl(arr: &mut ResultArray, values: &[c_int], count: c_int) {
    arr.count = if count < MAX_RESULTS as c_int {
        count
    } else {
        MAX_RESULTS as c_int
    };

    for i in 0..arr.count {
        let v = values[i as usize];
        arr.data[i as usize] = Result_ {
            value: v,
            scaled: (v as f64) * 1.5,
            rank: i,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    let arr = unsafe { &mut *arr };
    // The C code only ever reads `values[0 .. min(count, 10) - 1]`.
    let read_len = if count < MAX_RESULTS as c_int {
        count
    } else {
        MAX_RESULTS as c_int
    };
    let read_len = if read_len < 0 { 0 } else { read_len as usize };
    let values = unsafe { std::slice::from_raw_parts(values, read_len) };
    init_result_array_impl(arr, values, count);
}

/// Body of the `FOREACH` macro loop: visits `data[0 .. count-1]` once each.
fn process_with_foreach_impl(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;

    let count = arr.count;
    let mut count_iter: c_int = 0;
    while count_iter != count {
        let item = &mut arr.data[count_iter as usize];

        let result = op(item.value, item.rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = (result as f64) * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int_impl(temp);

        count_iter += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_with_foreach(
    arr: *mut ResultArray,
    op: OperationFuncOpt,
) -> c_int {
    let arr = unsafe { &mut *arr };
    let op = op.expect("null operation_func");
    process_with_foreach_impl(arr, op)
}

/// `weight` comes from `current - base`, a pointer difference in units of
/// `Result`, i.e. the element index; index 0 is not `> base` so it gets 1.
fn compute_weighted_sum_impl(arr: &ResultArray) -> c_int {
    let mut sum: c_int = 0;

    for i in 0..arr.count {
        let current = &arr.data[i as usize];

        let weight: c_int = if i > 0 { i } else { 1 };

        let weighted = (current.value as f64) * (weight as f64) * 0.8;
        sum = sum.wrapping_add(safe_double_to_int_impl(weighted));
    }

    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    compute_weighted_sum_impl(unsafe { &*arr })
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let operations: [OperationFunc; 4] = [
        add_operation,
        multiply_operation,
        subtract_operation,
        modulo_operation,
    ];

    let values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        param4.wrapping_div(2).wrapping_add(1),
    ];

    let mut arr = ResultArray::default();
    init_result_array_impl(&mut arr, &values, 8);

    let mut result: c_int = 0;

    for op in operations {
        result = result.wrapping_add(process_with_foreach_impl(&mut arr, op));
    }

    result = result.wrapping_add(compute_weighted_sum_impl(&arr));

    let mut i: c_int = 0;
    while i < arr.count - 1 {
        let cmp = compare_results_in_array_impl(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
        i += 1;
    }

    let final_scale = (result as f64) * 0.333;
    result = safe_double_to_int_impl(final_scale);

    result
}
