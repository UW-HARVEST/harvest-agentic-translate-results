use std::ffi::{c_double, c_int};

type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Result {
    value: c_int,
    padding_after_value: [u8; 4],
    scaled: c_double,
    rank: c_int,
    padding_after_rank: [u8; 4],
}

#[repr(C)]
pub struct ResultArray {
    data: [Result; 10],
    count: c_int,
}

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
    safe_double_to_int(base as c_double * scale_factor)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    let count = unsafe { (*arr).count };
    if idx1 >= count || idx2 >= count {
        return 0;
    }

    let data = unsafe { (*arr).data.as_mut_ptr() };
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
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    let stored_count = if count < 10 { count } else { 10 };
    unsafe {
        (*arr).count = stored_count;
    }

    for i in 0..stored_count {
        let value = unsafe { *values.offset(i as isize) };
        unsafe {
            let result = std::ptr::addr_of_mut!((*arr).data)
                .cast::<Result>()
                .add(i as usize);
            std::ptr::addr_of_mut!((*result).value).write(value);
            std::ptr::addr_of_mut!((*result).scaled).write(value as c_double * 1.5);
            std::ptr::addr_of_mut!((*result).rank).write(i);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_with_foreach(
    arr: *mut ResultArray,
    op: Option<OperationFunc>,
) -> c_int {
    let mut total: c_int = 0;
    let count = unsafe { (*arr).count };

    for i in 0..count {
        let item = unsafe { &mut (*arr).data[i as usize] };
        let result = unsafe { op.unwrap_unchecked()(item.value, item.rank, 0, 0) };
        total = total.wrapping_add(result);

        let temp = result as c_double * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);
    }

    total
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let mut sum: c_int = 0;
    let count = unsafe { (*arr).count };

    for i in 0..count {
        let current = unsafe { &(*arr).data[i as usize] };
        let weight = if i > 0 { i } else { 1 };
        let weighted = current.value as c_double * weight as c_double * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));
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

    let empty_result = Result {
        value: 0,
        padding_after_value: [0; 4],
        scaled: 0.0,
        rank: 0,
        padding_after_rank: [0; 4],
    };
    let mut arr = ResultArray {
        data: [empty_result; 10],
        count: 0,
    };
    unsafe {
        init_result_array(&mut arr, values.as_mut_ptr(), 8);
    }

    let mut result: c_int = 0;
    for op in operations {
        result = result.wrapping_add(unsafe { process_with_foreach(&mut arr, Some(op)) });
    }

    result = result.wrapping_add(unsafe { compute_weighted_sum(&mut arr) });

    for i in 0..arr.count - 1 {
        result = result.wrapping_add(unsafe { compare_results_in_array(&mut arr, i, i + 1) });
    }

    safe_double_to_int(result as c_double * 0.333)
}
