// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust - reproduces the byte-identical behavior of the C library.

use std::ffi::c_int;

const OCTAL_MASK_1: c_int = 0o777;
const OCTAL_MASK_2: c_int = 0o100;
const OCTAL_FLAG: c_int = 0o200;
const OCTAL_BASE: c_int = 0o10;

#[derive(Copy, Clone)]
#[repr(C)]
struct DataPoint {
    value: c_int,
    coefficient: f64,
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }

    if d >= c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d <= c_int::MIN as f64 {
        return c_int::MIN;
    }

    d as c_int
}

/// # Safety
/// `end` must point to the last valid element of an array of at least
/// `count` ints; the function reads `count` ints walking backwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *const c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = end;

    for _ in 0..count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = unsafe { ptr.offset(-1) };
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result = value;

    // Reproduce C switch fall-through semantics exactly.
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
    // C `malloc(size * sizeof(DataPoint))` with negative size will overflow
    // (or sign-extend) into a huge size_t and almost certainly fail -> NULL.
    // We match that behavior by returning -1 for negative sizes.
    if size < 0 {
        return -1;
    }

    let size_usz = size as usize;

    let mut points: Vec<DataPoint> = vec![
        DataPoint {
            value: 0,
            coefficient: 0.0,
        };
        size_usz
    ];

    for i in 0..size {
        points[i as usize].value = i.wrapping_mul(OCTAL_BASE);
        points[i as usize].coefficient = (i as f64) * multiplier;
    }

    let mut sum: f64 = 0.0;
    for i in 0..size {
        sum += (points[i as usize].value as f64) * points[i as usize].coefficient;
    }

    safe_double_to_int(sum)
}

/// # Safety
/// `array` must point to at least `count` valid c_int elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *const c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;

    for idx in 0..count {
        let element = unsafe { *array.offset(idx as isize) };
        total = total.wrapping_add(element);
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int;

    let base_value: c_int = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let mut data_array: Vec<c_int> = vec![0; array_size as usize];

    for i in 0..array_size {
        data_array[i as usize] = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
    }

    let foreach_result = unsafe { foreach_sum(data_array.as_ptr(), array_size) };

    // last_element pointer in C: data_array + array_size - 1.
    let last_index = (array_size - 1) as usize;
    let reverse_sum =
        unsafe { process_array_reverse(data_array.as_ptr().add(last_index), array_size) };

    // C `%` is truncated division; Rust's `%` (and wrapping_rem) match this.
    let switch_result = switch_fallthrough_calculator(param2, param3.wrapping_rem(5));

    let floating_calc: f64 =
        (param1 as f64) * 3.7 + (param2 as f64) * 2.3 - (param3 as f64) * 0.5;
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

    // data_array dropped automatically at end of scope (mimics free()).

    result &= OCTAL_MASK_1;

    result
}
