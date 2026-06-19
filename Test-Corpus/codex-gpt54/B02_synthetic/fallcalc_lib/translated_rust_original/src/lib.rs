use std::ffi::{c_double, c_int, c_void};
use std::mem::size_of;

const OCTAL_MASK_1: c_int = 0o777;
const OCTAL_MASK_2: c_int = 0o100;
const OCTAL_FLAG: c_int = 0o200;
const OCTAL_BASE: c_int = 0o10;

#[repr(C)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

fn safe_double_to_int_impl(d: c_double) -> c_int {
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

fn process_array_reverse_impl(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = end;
    let mut i = 0;

    while i < count {
        unsafe {
            sum = sum.wrapping_add(*ptr);
            ptr = ptr.sub(1);
        }
        i += 1;
    }

    sum
}

fn switch_fallthrough_calculator_impl(value: c_int, operation: c_int) -> c_int {
    let mut result = value;

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

fn allocate_and_compute_impl(size: c_int, multiplier: c_double) -> c_int {
    let alloc_size = (size as usize).wrapping_mul(size_of::<DataPoint>());
    let points = unsafe { libc::malloc(alloc_size) as *mut DataPoint };

    if points.is_null() {
        return -1;
    }

    let mut i = 0;
    while i < size {
        unsafe {
            (*points.add(i as usize)).value = i.wrapping_mul(OCTAL_BASE);
            (*points.add(i as usize)).coefficient = (i as c_double) * multiplier;
        }
        i += 1;
    }

    let mut sum = 0.0;
    let mut i = 0;
    while i < size {
        unsafe {
            let point = points.add(i as usize);
            sum += ((*point).value as c_double) * (*point).coefficient;
        }
        i += 1;
    }

    let result = safe_double_to_int_impl(sum);

    unsafe {
        libc::free(points as *mut c_void);
    }

    result
}

fn foreach_sum_impl(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;
    let mut idx: c_int = 0;

    while idx < count {
        unsafe {
            total = total.wrapping_add(*array.add(idx as usize));
        }
        idx += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    safe_double_to_int_impl(d)
}

#[unsafe(no_mangle)]
pub extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    process_array_reverse_impl(end, count)
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    switch_fallthrough_calculator_impl(value, operation)
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    allocate_and_compute_impl(size, multiplier)
}

#[unsafe(no_mangle)]
pub extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    foreach_sum_impl(array, count)
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let data_array =
        unsafe { libc::malloc((array_size as usize) * size_of::<c_int>()) as *mut c_int };

    if data_array.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < array_size {
        unsafe {
            *data_array.add(i as usize) = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        }
        i += 1;
    }

    let foreach_result = foreach_sum_impl(data_array, array_size);

    let last_element = unsafe { data_array.add((array_size - 1) as usize) };
    let reverse_sum = process_array_reverse_impl(last_element, array_size);

    let switch_result = switch_fallthrough_calculator_impl(param2, param3 % 5);

    let floating_calc =
        (param1 as c_double) * 3.7 + (param2 as c_double) * 2.3 - (param3 as c_double) * 0.5;
    let converted = safe_double_to_int_impl(floating_calc);

    let alloc_result = allocate_and_compute_impl(param4 % 10 + 1, 1.5);

    let mut result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    unsafe {
        libc::free(data_array as *mut c_void);
    }

    result &= OCTAL_MASK_1;

    result
}
