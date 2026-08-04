use std::os::raw::c_int;

#[derive(Clone, Copy)]
struct ResultItem {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

#[derive(Clone, Copy)]
struct ResultArray {
    data: [ResultItem; 10],
    count: c_int,
}

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

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
        a % b
    }
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= i32::MAX as f64 {
        i32::MAX
    } else if d <= i32::MIN as f64 {
        i32::MIN
    } else if d.is_nan() {
        0
    } else {
        d as c_int
    }
}

fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = (base as f64) * scale_factor;
    safe_double_to_int(scaled)
}

fn compare_results_in_array(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 < 0 || idx2 < 0 || idx1 >= arr.count || idx2 >= arr.count {
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
    let actual_count = count.clamp(0, 10) as usize;
    arr.count = actual_count as c_int;

    for (i, value) in values.iter().copied().take(actual_count).enumerate() {
        arr.data[i] = ResultItem {
            value,
            scaled: (value as f64) * 1.5,
            rank: i as c_int,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;
    let count = arr.count.max(0) as usize;

    for item in arr.data.iter_mut().take(count) {
        let result = op(item.value, item.rank, 0, 0);
        total = total.wrapping_add(result);
        let temp = (result as f64) * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);
    }

    total
}

fn compute_weighted_sum(arr: &ResultArray) -> c_int {
    let mut sum: c_int = 0;
    let count = arr.count.max(0) as usize;

    for i in 0..count {
        let current = &arr.data[i];
        let weight = if i > 0 { i as c_int } else { 1 };
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
        data: [
            ResultItem {
                value: 0,
                scaled: 0.0,
                rank: 0,
            };
            10
        ],
        count: 0,
    };

    init_result_array(&mut arr, &values, 8);

    let mut result: c_int = 0;

    for op in operations {
        result = result.wrapping_add(process_with_foreach(&mut arr, op));
    }

    result = result.wrapping_add(compute_weighted_sum(&arr));

    let upper = arr.count.saturating_sub(1);
    for i in 0..upper {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
    }

    let final_scale = (result as f64) * 0.333;
    let _ = compute_scaled_value(result, 0.333);
    safe_double_to_int(final_scale)
}
