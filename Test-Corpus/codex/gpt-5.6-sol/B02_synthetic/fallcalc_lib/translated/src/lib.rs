use std::ffi::{c_double, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const OCTAL_MASK_1: c_int = 0o777;
const OCTAL_MASK_2: c_int = 0o100;
const OCTAL_FLAG: c_int = 0o200;
const OCTAL_BASE: c_int = 0o10;

#[repr(C)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut current = end;
    let mut i: c_int = 0;

    while i < count {
        let value = unsafe { ptr::read(current) };
        sum = sum.wrapping_add(value);
        current = current.wrapping_offset(-1);
        i = i.wrapping_add(1);
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
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

#[unsafe(no_mangle)]
pub extern "C" fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    let allocation_size = (size as usize).wrapping_mul(size_of::<DataPoint>());
    let points = unsafe { malloc(allocation_size).cast::<DataPoint>() };

    if points.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < size {
        let point = DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: i as c_double * multiplier,
        };
        unsafe { ptr::write(points.wrapping_add(i as usize), point) };
        i = i.wrapping_add(1);
    }

    let mut sum = 0.0;
    i = 0;
    while i < size {
        let point = unsafe { &*points.wrapping_add(i as usize) };
        sum += point.value as c_double * point.coefficient;
        i = i.wrapping_add(1);
    }

    let result = safe_double_to_int(sum);
    unsafe { free(points.cast()) };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;
    let mut idx: c_int = 0;
    let size = count;

    while idx < size {
        let element = unsafe { ptr::read(array.wrapping_add(idx as usize)) };
        total = total.wrapping_add(element);
        idx = idx.wrapping_add(1);
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let allocation_size = (array_size as usize).wrapping_mul(size_of::<c_int>());
    let data_array = unsafe { malloc(allocation_size).cast::<c_int>() };

    if data_array.is_null() {
        return -1;
    }

    let mut i: c_int = 0;
    while i < array_size {
        let value = i
            .wrapping_add(1)
            .wrapping_mul(OCTAL_BASE)
            .wrapping_add(param1);
        unsafe { ptr::write(data_array.wrapping_add(i as usize), value) };
        i = i.wrapping_add(1);
    }

    let foreach_result = unsafe { foreach_sum(data_array, array_size) };
    let last_element = data_array.wrapping_add(array_size as usize - 1);
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };
    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc =
        param1 as c_double * 3.7 + param2 as c_double * 2.3 - param3 as c_double * 0.5;
    let converted = safe_double_to_int(floating_calc);
    let alloc_result = allocate_and_compute(param4 % 10 + 1, 1.5);

    let mut result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    unsafe { free(data_array.cast()) };

    result &= OCTAL_MASK_1;
    result
}
