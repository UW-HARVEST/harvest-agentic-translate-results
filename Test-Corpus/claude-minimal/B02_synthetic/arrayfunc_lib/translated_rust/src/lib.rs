// Rust translation of c_src/src/lib.c

type OperationFunc = fn(i32, i32, i32, i32) -> i32;

#[derive(Clone, Copy, Default)]
struct Result {
    value: i32,
    scaled: f64,
    rank: i32,
}

struct ResultArray {
    data: [Result; 10],
    count: i32,
}

fn add_operation(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a.wrapping_add(b)
}

fn multiply_operation(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a.wrapping_mul(b)
}

fn subtract_operation(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a.wrapping_sub(b)
}

fn modulo_operation(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    // Match C semantics: avoid overflow when a == INT_MIN and b == -1.
    if b == -1 {
        return 0;
    }
    a % b
}

fn safe_double_to_int(d: f64) -> i32 {
    if d.is_nan() {
        return 0;
    }
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    d as i32
}

#[allow(dead_code)]
fn compute_scaled_value(base: i32, scale_factor: f64) -> i32 {
    let scaled = base as f64 * scale_factor;
    safe_double_to_int(scaled)
}

fn compare_results_in_array(arr: &ResultArray, idx1: i32, idx2: i32) -> i32 {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }

    // In C this compared the addresses of arr->data[idx1] and arr->data[idx2].
    // Since both elements live in the same contiguous array, the relative
    // ordering of pointers matches the ordering of indices.
    if idx1 < idx2 {
        -1
    } else if idx1 > idx2 {
        1
    } else {
        0
    }
}

fn init_result_array(arr: &mut ResultArray, values: &[i32], count: i32) {
    arr.count = if count < 10 { count } else { 10 };

    for i in 0..arr.count as usize {
        arr.data[i] = Result {
            value: values[i],
            scaled: values[i] as f64 * 1.5,
            rank: i as i32,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> i32 {
    let mut total: i32 = 0;

    let count = arr.count as usize;
    for i in 0..count {
        let item = &mut arr.data[i];
        let result = op(item.value, item.rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = result as f64 * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);
    }

    total
}

fn compute_weighted_sum(arr: &ResultArray) -> i32 {
    let mut sum: i32 = 0;

    for i in 0..arr.count as usize {
        // In the original C: weight = (current > base) ? (current - base) : 1.
        // Here `current - base` is the index `i`. When i == 0, weight is 1.
        let weight: i32 = if i > 0 { i as i32 } else { 1 };

        let current = &arr.data[i];
        let weighted = current.value as f64 * weight as f64 * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));
    }

    sum
}

#[no_mangle]
pub extern "C" fn arrayfunc(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let operations: [OperationFunc; 4] = [
        add_operation,
        multiply_operation,
        subtract_operation,
        modulo_operation,
    ];

    let values: [i32; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        (param4 / 2).wrapping_add(1),
    ];

    let mut arr = ResultArray {
        data: [Result::default(); 10],
        count: 0,
    };
    init_result_array(&mut arr, &values, 8);

    let mut result: i32 = 0;

    for i in 0..4 {
        result = result.wrapping_add(process_with_foreach(&mut arr, operations[i]));
    }

    result = result.wrapping_add(compute_weighted_sum(&arr));

    for i in 0..(arr.count - 1) {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
    }

    let final_scale = result as f64 * 0.333;
    safe_double_to_int(final_scale)
}
