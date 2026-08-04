use std::ffi::c_int;

type OperationFunc = extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy)]
struct Result_ {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

#[repr(C)]
struct ResultArray {
    data: [Result_; 10],
    count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 { 0 } else { a.wrapping_rem(b) }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = base as f64 * scale_factor;
    safe_double_to_int(scaled)
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_results_in_array(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }
    let ptr1 = &arr.data[idx1 as usize] as *const Result_;
    let ptr2 = &arr.data[idx2 as usize] as *const Result_;
    if ptr1 < ptr2 {
        -1
    } else if ptr1 > ptr2 {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn init_result_array(arr: &mut ResultArray, values: *const c_int, count: c_int) {
    arr.count = if count < 10 { count } else { 10 };
    for i in 0..arr.count as usize {
        arr.data[i] = Result_ {
            value: unsafe { *values.add(i) },
            scaled: unsafe { *values.add(i) } as f64 * 1.5,
            rank: i as c_int,
        };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;
    for i in 0..arr.count as usize {
        let item = &arr.data[i];
        let result = op(item.value, item.rank, 0, 0);
        total = total.wrapping_add(result);
        let temp = result as f64 * 0.75;
        arr.data[i].scaled = temp;
        arr.data[i].value = safe_double_to_int(temp);
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_weighted_sum(arr: &ResultArray) -> c_int {
    let mut sum: c_int = 0;
    let base = &arr.data[0] as *const Result_;
    for i in 0..arr.count as usize {
        let current = &arr.data[i] as *const Result_;
        let weight = if current > base {
            unsafe { current.offset_from(base) as c_int }
        } else {
            1
        };
        let weighted = arr.data[i].value as f64 * weight as f64 * 0.8;
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

    let values = [
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
        data: [Result_ { value: 0, scaled: 0.0, rank: 0 }; 10],
        count: 0,
    };
    init_result_array(&mut arr, values.as_ptr(), 8);

    let mut result: c_int = 0;

    for i in 0..4 {
        result = result.wrapping_add(process_with_foreach(&mut arr, operations[i]));
    }

    result = result.wrapping_add(compute_weighted_sum(&arr));

    for i in 0..(arr.count - 1) as usize {
        let cmp = compare_results_in_array(&arr, i as c_int, (i + 1) as c_int);
        result = result.wrapping_add(cmp);
    }

    let final_scale = result as f64 * 0.333;
    result = safe_double_to_int(final_scale);

    result
}
