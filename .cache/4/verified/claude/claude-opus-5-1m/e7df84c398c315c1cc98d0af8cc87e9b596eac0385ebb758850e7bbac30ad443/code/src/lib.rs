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

#![allow(non_snake_case)]

use core::ffi::{c_double, c_int, c_void};

// ---------------------------------------------------------------------------
// Macros / constants from lib.c
// ---------------------------------------------------------------------------

const OCTAL_MASK_1: c_int = 0o777; // 511
const OCTAL_MASK_2: c_int = 0o100; // 64
const OCTAL_FLAG: c_int = 0o200; // 128
const OCTAL_BASE: c_int = 0o10; // 8

// <limits.h>
const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;

// ---------------------------------------------------------------------------
// typedef struct { int value; double coefficient; } DataPoint;
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

// We use the platform C allocator so that allocation behaviour (including
// failure for absurd/negative sizes and the non-NULL result of malloc(0))
// matches the original C library byte for byte.
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
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

    // At this point INT_MIN < d < INT_MAX, so the C cast (truncation toward
    // zero) is well defined and cannot saturate.
    d as c_int
}

// ---------------------------------------------------------------------------
// int process_array_reverse(int *end, int count)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr: *mut c_int = end;

    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = unsafe { ptr.offset(-1) };
        i += 1;
    }

    sum
}

// ---------------------------------------------------------------------------
// int switch_fallthrough_calculator(int value, int operation)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result: c_int = value;

    // The original C switch relies on fall-through between the cases; the
    // structure below reproduces exactly the same sequence of operations.
    match operation {
        0 => {
            // case 0: falls through to 1 and 2
            result = result.wrapping_mul(OCTAL_BASE);
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        1 => {
            // case 1: falls through to 2
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        2 => {
            result &= OCTAL_MASK_1;
        }
        3 => {
            // case 3: falls through to 4
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
    // malloc(size * sizeof(DataPoint)) -- the int is converted to size_t
    // before the multiplication, exactly as C does, so negative sizes wrap
    // around to enormous requests (which fail).
    let bytes = (size as usize).wrapping_mul(core::mem::size_of::<DataPoint>());
    let points = unsafe { malloc(bytes) } as *mut DataPoint;

    if points.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < size {
        unsafe {
            (*points.offset(i as isize)).value = i.wrapping_mul(OCTAL_BASE);
            (*points.offset(i as isize)).coefficient = (i as c_double) * multiplier;
        }
        i += 1;
    }

    let mut sum: c_double = 0.0;
    let mut i: c_int = 0;
    while i < size {
        let p = unsafe { *points.offset(i as isize) };
        sum += (p.value as c_double) * p.coefficient;
        i += 1;
    }

    let result = safe_double_to_int(sum);

    unsafe { free(points as *mut c_void) };

    result
}

// ---------------------------------------------------------------------------
// int foreach_sum(int *array, int count)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;

    // FOREACH(element, array, count) { total += element; }
    let size = count;
    let mut idx: c_int = 0;
    while idx < size {
        let element = unsafe { *array.offset(idx as isize) };
        total = total.wrapping_add(element);
        idx += 1;
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
    let bytes = (array_size as usize).wrapping_mul(core::mem::size_of::<c_int>());
    let data_array = unsafe { malloc(bytes) } as *mut c_int;

    if data_array.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < array_size {
        unsafe {
            *data_array.offset(i as isize) =
                (i.wrapping_add(1)).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        }
        i += 1;
    }

    let foreach_result = unsafe { foreach_sum(data_array, array_size) };

    let last_element = unsafe { data_array.offset((array_size - 1) as isize) };
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };

    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc: c_double =
        (param1 as c_double) * 3.7 + (param2 as c_double) * 2.3 - (param3 as c_double) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute(param4 % 10 + 1, 1.5);

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
