use std::ffi::c_int;

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResultItem {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ResultArray {
    data: [ResultItem; 10],
    count: c_int,
}

impl Default for ResultArray {
    fn default() -> Self {
        Self {
            data: [ResultItem::default(); 10],
            count: 0,
        }
    }
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
        return 0;
    }
    a % b
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= f64::from(i32::MAX) {
        return i32::MAX;
    }
    if d <= f64::from(i32::MIN) {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    d as c_int
}

#[allow(dead_code)]
fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = f64::from(base) * scale_factor;
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

    for i in 0..(arr.count as usize) {
        arr.data[i] = ResultItem {
            value: values[i],
            scaled: f64::from(values[i]) * 1.5,
            rank: i as c_int,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;
    let size = arr.count;
    let mut count_iter = 0;

    while count_iter != size {
        let item = &mut arr.data[count_iter as usize];
        let result = op(item.value, item.rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = f64::from(result) * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);

        count_iter += 1;
    }

    total
}

fn compute_weighted_sum(arr: &ResultArray) -> c_int {
    let mut sum: c_int = 0;

    for i in 0..(arr.count as usize) {
        let current = &arr.data[i];
        let weight = if i > 0 { i as c_int } else { 1 };
        let weighted = f64::from(current.value) * f64::from(weight) * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
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

    let mut arr = ResultArray::default();
    init_result_array(&mut arr, &values, 8);

    let mut result: c_int = 0;

    for operation in operations {
        result = result.wrapping_add(process_with_foreach(&mut arr, operation));
    }

    result = result.wrapping_add(compute_weighted_sum(&arr));

    let mut i = 0;
    while i < arr.count - 1 {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
        i += 1;
    }

    let final_scale = f64::from(result) * 0.333;
    safe_double_to_int(final_scale)
}
