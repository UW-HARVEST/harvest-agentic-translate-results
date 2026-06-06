// Rust translation of c_src/src/lib.c
// Preserves the original behavior byte-identically.

use std::ffi::c_int;

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Copy, Clone)]
struct Result {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

struct ResultArray {
    data: [Result; 10],
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
        return 0;
    }
    // C uses truncated modulo, which Rust's `%` also does for signed integers.
    // Avoid panic on INT_MIN % -1 by using wrapping_rem.
    a.wrapping_rem(b)
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    // Truncation toward zero, matching C's `(int)d` cast.
    d as c_int
}

#[allow(dead_code)]
fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = base as f64 * scale_factor;
    safe_double_to_int(scaled)
}

fn compare_results_in_array(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }

    // The original C code compares pointers `&arr->data[idx1]` vs `&arr->data[idx2]`
    // which, since they live in the same array, is equivalent to comparing the indices.
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

    for i in 0..arr.count as usize {
        arr.data[i] = Result {
            value: values[i],
            scaled: values[i] as f64 * 1.5,
            rank: i as c_int,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;
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

fn compute_weighted_sum(arr: &mut ResultArray) -> c_int {
    let mut sum: c_int = 0;
    let count = arr.count as usize;

    for i in 0..count {
        // In C: weight = (current > base) ? (current - base) : 1
        // Pointer subtraction yields the index difference (in elements).
        let weight: c_int = if i > 0 { i as c_int } else { 1 };

        let current_value = arr.data[i].value;
        let weighted = current_value as f64 * weight as f64 * 0.8;
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

    let values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        // C: param4 / 2 + 1 — integer division truncates toward zero, matching Rust's `/`.
        // Use wrapping ops to avoid panics on edge cases.
        (param4.wrapping_div(2)).wrapping_add(1),
    ];

    let mut arr = ResultArray {
        data: [Result {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
        count: 0,
    };
    init_result_array(&mut arr, &values, 8);

    let mut result: c_int = 0;

    for i in 0..4 {
        result = result.wrapping_add(process_with_foreach(&mut arr, operations[i]));
    }

    result = result.wrapping_add(compute_weighted_sum(&mut arr));

    let mut i: c_int = 0;
    while i < arr.count - 1 {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
        i += 1;
    }

    let final_scale = result as f64 * 0.333;
    result = safe_double_to_int(final_scale);

    result
}
