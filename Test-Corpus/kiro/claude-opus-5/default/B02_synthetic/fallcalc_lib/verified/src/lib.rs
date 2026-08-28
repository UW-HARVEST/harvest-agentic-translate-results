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
//! The C translation unit declares every function with external linkage (none
//! are `static`), and the header only advertises `fallcalc`. There are no
//! namespace-renaming preprocessor macros, so the linker symbols are the plain
//! source-level names. All of them are re-exported here with the same symbols
//! and signatures.

use std::ffi::c_int;

// #define OCTAL_MASK_1 0777
const OCTAL_MASK_1: c_int = 0o777;
// #define OCTAL_MASK_2 0100
const OCTAL_MASK_2: c_int = 0o100;
// #define OCTAL_FLAG   0200
const OCTAL_FLAG: c_int = 0o200;
// #define OCTAL_BASE   010
const OCTAL_BASE: c_int = 0o10;

/// `typedef struct { int value; double coefficient; } DataPoint;`
#[repr(C)]
#[derive(Clone, Copy)]
struct DataPoint {
    value: c_int,
    coefficient: f64,
}

/// Number of bytes `malloc` is asked for by
/// `malloc(size * sizeof(DataPoint))`, reproducing the C conversion of a
/// possibly negative `int` to `size_t`.
fn malloc_byte_request(count: c_int) -> usize {
    (count as isize as usize).wrapping_mul(size_of::<DataPoint>())
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }

    // (double)INT_MAX and (double)INT_MIN are both exactly representable.
    if d >= c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d <= c_int::MIN as f64 {
        return c_int::MIN;
    }

    // In range, so the C cast is a plain truncation toward zero.
    d as c_int
}

/// Walks *backwards* from `end`, reading `count` elements.
///
/// # Safety
///
/// Mirrors the C contract: `end` must be the last element of a block of at
/// least `count` `int`s when `count > 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = end;

    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = unsafe { ptr.offset(-1) };
        i += 1;
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result: c_int = value;

    // The C `switch` falls through from 0 -> 1 -> 2 and from 3 -> 4.
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

#[unsafe(no_mangle)]
pub extern "C" fn allocate_and_compute(size: c_int, multiplier: f64) -> c_int {
    // `malloc(size * sizeof(DataPoint))`: a negative `size` becomes an enormous
    // `size_t`, so the allocation fails and the C returns -1. A `size` of 0
    // yields a non-NULL pointer from malloc(0), so the C falls through with an
    // empty array.
    let bytes = malloc_byte_request(size);
    if bytes > isize::MAX as usize {
        return -1;
    }

    let elements = size as usize;
    let mut points: Vec<DataPoint> = Vec::new();
    if points.try_reserve_exact(elements).is_err() {
        return -1;
    }

    for i in 0..size {
        points.push(DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: i as f64 * multiplier,
        });
    }

    let mut sum: f64 = 0.0;
    for i in 0..size as usize {
        sum += points[i].value as f64 * points[i].coefficient;
    }

    // `free(points)` is the Vec drop.
    safe_double_to_int(sum)
}

/// Reproduces the `FOREACH` macro, whose nested-loop / `keep` toggling walks
/// the array once from index 0 to `count - 1`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;

    let size = count;
    let mut keep = true;
    let mut idx: c_int = 0;
    while keep && idx < size {
        let element = unsafe { *array.offset(idx as isize) };
        while keep {
            total = total.wrapping_add(element);
            keep = !keep;
        }
        keep = !keep;
        idx += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int;

    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let mut data_array: Vec<c_int> = Vec::new();
    if data_array.try_reserve_exact(array_size as usize).is_err() {
        return -1;
    }
    data_array.resize(array_size as usize, 0);

    for i in 0..array_size {
        data_array[i as usize] = (i.wrapping_add(1))
            .wrapping_mul(OCTAL_BASE)
            .wrapping_add(param1);
    }

    let base_ptr = data_array.as_mut_ptr();

    let foreach_result = unsafe { foreach_sum(base_ptr, array_size) };

    let last_element = unsafe { base_ptr.offset(array_size as isize - 1) };
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };

    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc = param1 as f64 * 3.7 + param2 as f64 * 2.3 - param3 as f64 * 0.5;
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

    // `free(data_array)` is the Vec drop.
    drop(data_array);

    result &= OCTAL_MASK_1;

    result
}
