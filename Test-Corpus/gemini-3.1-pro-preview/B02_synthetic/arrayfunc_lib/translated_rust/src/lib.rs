use std::os::raw::c_int;

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Clone, Copy, Default)]
struct CalcResult {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

struct ResultArray {
    data: [CalcResult; 10],
    count: c_int,
}

fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

fn multiply_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_mul(b)
}

fn subtract_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_sub(b)
}

fn modulo_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a.wrapping_rem(b)
    }
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d <= c_int::MIN as f64 {
        return c_int::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as c_int
}

#[allow(dead_code)]
fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = (base as f64) * scale_factor;
    safe_double_to_int(scaled)
}

fn compare_results_in_array(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }
    if idx1 < idx2 {
        -1
    } else if idx1 > idx2 {
        1
    } else {
        0
    }
}

fn init_result_array(arr: &mut ResultArray, values: &[c_int], count: c_int) {
    arr.count = if count < 10 { count } else { 10 };
    for i in 0..arr.count {
        let idx = i as usize;
        arr.data[idx] = CalcResult {
            value: values[idx],
            scaled: (values[idx] as f64) * 1.5,
            rank: i,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total = 0;
    for i in 0..arr.count {
        let item = &mut arr.data[i as usize];
        let result = op(item.value, item.rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = (result as f64) * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);
    }
    total
}

fn compute_weighted_sum(arr: &ResultArray) -> c_int {
    let mut sum = 0;
    for i in 0..arr.count {
        let current = &arr.data[i as usize];
        let weight = if i > 0 { i } else { 1 };

        let weighted = (current.value as f64) * (weight as f64) * 0.8;
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
        param4 / 2 + 1,
    ];

    let mut arr = ResultArray {
        data: [CalcResult::default(); 10],
        count: 0,
    };
    init_result_array(&mut arr, &values, 8);

    let mut result = 0;

    for i in 0..4 {
        result = result.wrapping_add(process_with_foreach(&mut arr, operations[i]));
    }

    result = result.wrapping_add(compute_weighted_sum(&arr));

    for i in 0..(arr.count - 1) {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
    }

    let final_scale = (result as f64) * 0.333;
    result = safe_double_to_int(final_scale);

    result
}
