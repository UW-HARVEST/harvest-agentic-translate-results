// Rust translation of c_src/src/lib.c
//
// Original C copyright header:
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

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_double, c_int, c_void};

// ---------------------------------------------------------------------------
// C preprocessor constants (octal literals in the original source)
// ---------------------------------------------------------------------------
const OCTAL_MASK_1: c_int = 0o777; // 511
const OCTAL_MASK_2: c_int = 0o100; // 64
const OCTAL_FLAG: c_int = 0o200; // 128
const OCTAL_BASE: c_int = 0o10; // 8

// `limits.h`
const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;

// ---------------------------------------------------------------------------
// The C code uses malloc/free from libc directly; we bind to the very same
// allocator so that allocation-failure behaviour (and hence return values)
// matches bit for bit.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// typedef struct { int value; double coefficient; } DataPoint;
#[repr(C)]
#[derive(Clone, Copy)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

// ---------------------------------------------------------------------------
// int safe_double_to_int(double d)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { INT_MAX } else { INT_MIN };
    }

    if d >= INT_MAX as c_double {
        return INT_MAX;
    }
    if d <= INT_MIN as c_double {
        return INT_MIN;
    }

    // C truncating conversion; the guards above keep this in range.
    d as c_int
}

// ---------------------------------------------------------------------------
// int process_array_reverse(int *end, int count)
//
// Walks *backwards* from `end`, summing `count` elements.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr: *mut c_int = end;

    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = unsafe { ptr.offset(-1) };
        i = i.wrapping_add(1);
    }

    sum
}

// ---------------------------------------------------------------------------
// int switch_fallthrough_calculator(int value, int operation)
//
// The original switch deliberately falls through:
//   0 -> *=8, +=128, &=511
//   1 ->      +=128, &=511
//   2 ->             &=511
//   3 -> *=3,  +=64
//   4 ->       +=64
//   default -> 0
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result: c_int = value;

    match operation {
        0 => {
            result = result.wrapping_mul(OCTAL_BASE);
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        1 => {
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        2 => {
            result &= OCTAL_MASK_1;
        }
        3 => {
            result = result.wrapping_mul(3);
            result = result.wrapping_add(OCTAL_MASK_2);
        }
        4 => {
            result = result.wrapping_add(OCTAL_MASK_2);
        }
        _ => {
            result = 0;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// int allocate_and_compute(int size, double multiplier)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    // C: malloc(size * sizeof(DataPoint)) -- `size` is promoted to size_t, so a
    // negative `size` becomes an enormous unsigned request and malloc fails.
    let bytes: usize = (size as isize as usize).wrapping_mul(core::mem::size_of::<DataPoint>());
    let points = unsafe { malloc(bytes) } as *mut DataPoint;

    if points.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < size {
        unsafe {
            let p = points.offset(i as isize);
            (*p).value = i.wrapping_mul(OCTAL_BASE);
            (*p).coefficient = (i as c_double) * multiplier;
        }
        i = i.wrapping_add(1);
    }

    let mut sum: c_double = 0.0;
    let mut i: c_int = 0;
    while i < size {
        unsafe {
            let p = points.offset(i as isize);
            sum += (*p).value as c_double * (*p).coefficient;
        }
        i = i.wrapping_add(1);
    }

    let result = safe_double_to_int(sum);

    unsafe { free(points as *mut c_void) };

    result
}

// ---------------------------------------------------------------------------
// int foreach_sum(int *array, int count)
//
// The FOREACH macro expands to a double `for` loop that visits every element
// exactly once, in order.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;

    // Faithful expansion of the FOREACH macro.
    let size: c_int = count;
    let mut keep: c_int = 1;
    let mut idx: c_int = 0;
    while keep != 0 && idx < size {
        let element: c_int = unsafe { *array.offset(idx as isize) };
        while keep != 0 {
            total = total.wrapping_add(element);
            keep = (keep == 0) as c_int;
        }
        keep = (keep == 0) as c_int;
        idx = idx.wrapping_add(1);
    }

    total
}

// ---------------------------------------------------------------------------
// int fallcalc(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int;

    let base_value: c_int = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let data_array =
        unsafe { malloc((array_size as usize) * core::mem::size_of::<c_int>()) } as *mut c_int;

    if data_array.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < array_size {
        unsafe {
            *data_array.offset(i as isize) =
                i.wrapping_add(1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        }
        i = i.wrapping_add(1);
    }

    let foreach_result = unsafe { foreach_sum(data_array, array_size) };

    let last_element =
        unsafe { data_array.offset(array_size as isize).offset(-1) };
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };

    let switch_result = switch_fallthrough_calculator(param2, param3.wrapping_rem(5));

    let floating_calc: c_double =
        (param1 as c_double) * 3.7 + (param2 as c_double) * 2.3 - (param3 as c_double) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute(param4.wrapping_rem(10).wrapping_add(1), 1.5);

    result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    unsafe { free(data_array as *mut c_void) };

    result &= OCTAL_MASK_1;

    result
}
