use std::ffi::{c_double, c_int};

type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Result {
    value: c_int,
    scaled: c_double,
    rank: c_int,
}

#[repr(C)]
pub struct ResultArray {
    data: [Result; 10],
    count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int {
    let _ = unused1;
    let _ = unused2;
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int {
    let _ = unused1;
    let _ = unused2;
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int {
    let _ = unused1;
    let _ = unused2;
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, unused1: c_int, unused2: c_int) -> c_int {
    let _ = unused1;
    let _ = unused2;
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }
    if d != d {
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
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    unsafe {
        if idx1 >= (*arr).count || idx2 >= (*arr).count {
            return 0;
        }

        let ptr1 = (*arr).data.as_mut_ptr().offset(idx1 as isize);
        let ptr2 = (*arr).data.as_mut_ptr().offset(idx2 as isize);

        if ptr1 < ptr2 {
            -1
        } else if ptr1 > ptr2 {
            1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    unsafe {
        (*arr).count = if count < 10 { count } else { 10 };

        for i in 0..(*arr).count {
            let value = *values.offset(i as isize);
            (*arr).data[i as usize] = Result {
                value,
                scaled: value as c_double * 1.5,
                rank: i,
            };
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_with_foreach(arr: *mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;

    unsafe {
        let size = (*arr).count;
        let mut count_iter: c_int = 0;
        while count_iter != size {
            let item = (*arr).data.as_mut_ptr().offset(count_iter as isize);
            let result = op.unwrap_unchecked()((*item).value, (*item).rank, 0, 0);
            total = total.wrapping_add(result);

            let temp = result as c_double * 0.75;
            (*item).scaled = temp;
            (*item).value = safe_double_to_int(temp);

            count_iter = count_iter.wrapping_add(1);
        }
    }

    total
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let mut sum: c_int = 0;

    unsafe {
        for i in 0..(*arr).count {
            let current = (*arr).data.as_mut_ptr().offset(i as isize);
            let base = (*arr).data.as_mut_ptr();

            let weight = if current > base {
                current.offset_from(base) as c_int
            } else {
                1
            };

            let weighted = (*current).value as c_double * weight as c_double * 0.8;
            sum = sum.wrapping_add(safe_double_to_int(weighted));
        }
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let operations: [unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int; 4] = [
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
        param4.wrapping_div(2).wrapping_add(1),
    ];

    let mut arr = ResultArray {
        data: [Result {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
        count: 0,
    };

    unsafe {
        init_result_array(&mut arr, values.as_mut_ptr(), 8);
    }

    let mut result: c_int = 0;

    for operation in operations {
        unsafe {
            result = result.wrapping_add(process_with_foreach(&mut arr, Some(operation)));
        }
    }

    unsafe {
        result = result.wrapping_add(compute_weighted_sum(&mut arr));
    }

    for i in 0..arr.count - 1 {
        unsafe {
            let cmp = compare_results_in_array(&mut arr, i, i + 1);
            result = result.wrapping_add(cmp);
        }
    }

    let final_scale = result as c_double * 0.333;
    safe_double_to_int(final_scale)
}
