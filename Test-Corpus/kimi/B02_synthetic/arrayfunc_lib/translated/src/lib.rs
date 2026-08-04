use std::os::raw::c_int;

#[repr(C)]
pub struct Result {
    pub value: c_int,
    pub scaled: f64,
    pub rank: c_int,
}

#[repr(C)]
pub struct ResultArray {
    pub data: [Result; 10],
    pub count: c_int,
}

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a + b
}

fn multiply_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a * b
}

fn subtract_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a - b
}

fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a % b
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as c_int
}

fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = base as f64 * scale_factor;
    safe_double_to_int(scaled)
}

fn compare_results_in_array(arr: &ResultArray, idx1: usize, idx2: usize) -> c_int {
    if idx1 >= arr.count as usize || idx2 >= arr.count as usize {
        return 0;
    }

    let ptr1 = &arr.data[idx1] as *const Result;
    let ptr2 = &arr.data[idx2] as *const Result;

    if ptr1 < ptr2 {
        -1
    } else if ptr1 > ptr2 {
        1
    } else {
        0
    }
}

fn init_result_array(arr: &mut ResultArray, values: &[c_int], count: usize) {
    let actual_count = if count < 10 { count } else { 10 };
    arr.count = actual_count as c_int;

    for i in 0..actual_count {
        arr.data[i] = Result {
            value: values[i],
            scaled: values[i] as f64 * 1.5,
            rank: i as c_int,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total = 0;

    for i in 0..arr.count as usize {
        let item = &mut arr.data[i];
        let result = op(item.value, item.rank, 0, 0);
        total += result;

        let temp = result as f64 * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);
    }

    total
}

fn compute_weighted_sum(arr: &ResultArray) -> c_int {
    let mut sum = 0;

    for i in 0..arr.count as usize {
        let current = &arr.data[i];
        let base = &arr.data[0];

        let current_ptr = current as *const Result;
        let base_ptr = base as *const Result;

        let weight = if current_ptr > base_ptr {
            (current_ptr as usize - base_ptr as usize) / std::mem::size_of::<Result>()
        } else {
            1
        };

        let weighted = current.value as f64 * weight as f64 * 0.8;
        sum += safe_double_to_int(weighted);
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

    let values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1 + param2,
        param2 - param3,
        param3 * 2,
        param4 / 2 + 1,
    ];

    let mut arr = ResultArray {
        data: [
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
            Result { value: 0, scaled: 0.0, rank: 0 },
        ],
        count: 0,
    };
    init_result_array(&mut arr, &values, 8);

    let mut result = 0;

    for i in 0..4 {
        result += process_with_foreach(&mut arr, operations[i]);
    }

    result += compute_weighted_sum(&arr);

    for i in 0..(arr.count as usize - 1) {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result += cmp;
    }

    let final_scale = result as f64 * 0.333;
    result = safe_double_to_int(final_scale);

    result
}
