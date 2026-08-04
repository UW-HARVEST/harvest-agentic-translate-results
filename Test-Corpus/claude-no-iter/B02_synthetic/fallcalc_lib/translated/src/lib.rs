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

use std::os::raw::{c_double, c_int};

const OCTAL_MASK_1: c_int = 0o777; // 511
const OCTAL_MASK_2: c_int = 0o100; // 64
const OCTAL_FLAG: c_int = 0o200; // 128
const OCTAL_BASE: c_int = 0o10; // 8

#[derive(Clone, Copy)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

fn safe_double_to_int(d: c_double) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }

    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }

    d as c_int
}

fn process_array_reverse(arr: &[c_int]) -> c_int {
    // The original C iterates from the end backwards, but the result is
    // the sum of all elements in the slice. Integer addition (with
    // wrapping semantics) is associative, so we get the identical value
    // regardless of iteration order.
    let mut sum: c_int = 0;
    for &v in arr.iter().rev() {
        sum = sum.wrapping_add(v);
    }
    sum
}

fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result: c_int = value;

    // Faithfully reproduce the C switch with intentional fall-through.
    match operation {
        0 => {
            // case 0 falls through to 1, 2
            result = result.wrapping_mul(OCTAL_BASE);
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        1 => {
            // case 1 falls through to 2
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        2 => {
            result &= OCTAL_MASK_1;
        }
        3 => {
            // case 3 falls through to 4
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

fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    // The C code performs `malloc(size * sizeof(DataPoint))`. When `size`
    // is negative, the computed byte count wraps to a huge `size_t`
    // value and `malloc` typically returns NULL, causing the function
    // to return -1. Reproduce that behavior here for negative sizes.
    if size < 0 {
        return -1;
    }

    let size_usize = size as usize;
    let mut points: Vec<DataPoint> = Vec::with_capacity(size_usize);

    let mut i: c_int = 0;
    while i < size {
        points.push(DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: (i as c_double) * multiplier,
        });
        i += 1;
    }

    let mut sum: c_double = 0.0;
    for p in &points {
        sum += (p.value as c_double) * p.coefficient;
    }

    safe_double_to_int(sum)
}

fn foreach_sum(arr: &[c_int]) -> c_int {
    let mut total: c_int = 0;
    for &element in arr {
        total = total.wrapping_add(element);
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let _result_init: c_int = 0;
    let _ = _result_init;

    let base_value: c_int = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let mut data_array: Vec<c_int> = Vec::with_capacity(array_size as usize);

    let mut i: c_int = 0;
    while i < array_size {
        let v = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        data_array.push(v);
        i += 1;
    }

    let foreach_result: c_int = foreach_sum(&data_array);

    // In C: pointer to last element, then walk backwards `array_size`
    // times. The values summed are exactly the slice contents.
    let reverse_sum: c_int = process_array_reverse(&data_array);

    let switch_result: c_int = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc: c_double =
        (param1 as c_double) * 3.7 + (param2 as c_double) * 2.3 - (param3 as c_double) * 0.5;
    let converted: c_int = safe_double_to_int(floating_calc);

    let alloc_result: c_int = allocate_and_compute(param4 % 10 + 1, 1.5);

    let mut result: c_int = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    // data_array is dropped here (matches `free(data_array)` in C).
    drop(data_array);

    result &= OCTAL_MASK_1;

    result
}
