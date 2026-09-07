use std::ffi::{c_double, c_int};

#[repr(C)]
pub struct Result {
    pub value: c_int,
    pub scaled: c_double,
    pub rank: c_int,
}

#[repr(C)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

pub type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(
    a: c_int,
    b: c_int,
    _unused1: c_int,
    _unused2: c_int,
) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(
    a: c_int,
    b: c_int,
    _unused1: c_int,
    _unused2: c_int,
) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: c_double) -> c_int {
    let scaled = base as c_double * scale_factor;
    safe_double_to_int(scaled)
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `arr` must point to a valid `ResultArray`, and in-range indices must
/// identify elements of `arr.data`.
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    let count = unsafe { (*arr).count };
    if idx1 >= count || idx2 >= count {
        return 0;
    }

    let data = unsafe { std::ptr::addr_of_mut!((*arr).data).cast::<Result>() };
    let ptr1 = data.wrapping_offset(idx1 as isize);
    let ptr2 = data.wrapping_offset(idx2 as isize);

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
///
/// `arr` must be writable, and `values` must contain at least
/// `min(count, 10)` readable elements when `count` is positive.
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    let stored_count = if count < 10 { count } else { 10 };
    unsafe {
        (*arr).count = stored_count;
    }

    let mut i = 0;
    while i < stored_count {
        let value = unsafe { *values.offset(i as isize) };
        unsafe {
            std::ptr::addr_of_mut!((*arr).data)
                .cast::<Result>()
                .offset(i as isize)
                .write(Result {
                    value,
                    scaled: value as c_double * 1.5,
                    rank: i,
                });
        }
        i += 1;
    }
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `arr` must point to a writable `ResultArray` whose count is valid for
/// `data`, and `op` must be callable with the declared C ABI.
pub unsafe extern "C" fn process_with_foreach(arr: *mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;
    let mut count_iter: c_int = 0;
    let size = unsafe { (*arr).count };
    let data = unsafe { std::ptr::addr_of_mut!((*arr).data).cast::<Result>() };

    while count_iter != size {
        let item = data.wrapping_offset(count_iter as isize);
        let result = unsafe { op((*item).value, (*item).rank, 0, 0) };
        total = total.wrapping_add(result);

        let temp = result as c_double * 0.75;
        unsafe {
            (*item).scaled = temp;
            (*item).value = safe_double_to_int(temp);
        }
        count_iter = count_iter.wrapping_add(1);
    }

    total
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `arr` must point to a `ResultArray` whose count is valid for `data`.
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    let count = unsafe { (*arr).count };
    let data = unsafe { std::ptr::addr_of_mut!((*arr).data).cast::<Result>() };

    while i < count {
        let current = data.wrapping_offset(i as isize);
        let base = data;
        let weight = if current > base { i } else { 1 };
        let value = unsafe { (*current).value };
        let weighted = value as c_double * weight as c_double * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));
        i += 1;
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let operations: [OperationFunc; 4] = [
        add_operation,
        multiply_operation,
        subtract_operation,
        modulo_operation,
    ];
    let mut values = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        param4 / 2 + 1,
    ];
    let mut arr = ResultArray {
        data: std::array::from_fn(|_| Result {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }),
        count: 0,
    };
    unsafe {
        init_result_array(&mut arr, values.as_mut_ptr(), 8);
    }

    let mut result: c_int = 0;
    for op in operations {
        result = result.wrapping_add(unsafe { process_with_foreach(&mut arr, op) });
    }
    result = result.wrapping_add(unsafe { compute_weighted_sum(&mut arr) });

    let mut i = 0;
    while i < arr.count - 1 {
        result = result.wrapping_add(unsafe { compare_results_in_array(&mut arr, i, i + 1) });
        i += 1;
    }

    let final_scale = result as c_double * 0.333;
    safe_double_to_int(final_scale)
}
