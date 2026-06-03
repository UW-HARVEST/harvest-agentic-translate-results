// SPDX-License-Identifier: MIT
// Rust translation of c_src/src/lib.c

use std::ffi::c_int;

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Clone, Copy)]
struct Result_ {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

impl Result_ {
    const fn zero() -> Self {
        Result_ {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }
    }
}

struct ResultArray {
    data: [Result_; 10],
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
    // Match C's `%` semantics. Avoid overflow on INT_MIN % -1 (which is UB in C
    // and panics in Rust); the C original does not guard this, but it is the
    // only undefined case for the operator. Use wrapping behaviour to mirror
    // typical C runtime output without panicking.
    if a == c_int::MIN && b == -1 {
        return 0;
    }
    a % b
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d <= c_int::MIN as f64 {
        return c_int::MIN;
    }
    if d != d {
        return 0;
    }
    // C cast of a finite, in-range double to int truncates toward zero.
    // Rust's `as i32` from f64 saturates on out-of-range, but we've already
    // bounded d to (INT_MIN, INT_MAX), so behaviour matches.
    d as c_int
}

fn compare_results_in_array(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }

    // ptr1 = &arr.data[idx1], ptr2 = &arr.data[idx2]; compare addresses.
    // Since both elements live in the same array, address ordering matches
    // index ordering.
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
        arr.data[idx] = Result_ {
            value: values[idx],
            scaled: (values[idx] as f64) * 1.5,
            rank: i,
        };
    }
}

fn process_with_foreach(arr: &mut ResultArray, op: OperationFunc) -> c_int {
    let mut total: c_int = 0;

    // FOREACH macro iterates `count_iter` from 0 to `size`, with `item`
    // pointing at `array + count_iter`. Replicate by iterating over indices.
    let size = arr.count;
    for count_iter in 0..size {
        let idx = count_iter as usize;
        let value = arr.data[idx].value;
        let rank = arr.data[idx].rank;
        let result = op(value, rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = (result as f64) * 0.75;
        arr.data[idx].scaled = temp;
        arr.data[idx].value = safe_double_to_int(temp);
    }

    total
}

fn compute_weighted_sum(arr: &mut ResultArray) -> c_int {
    let mut sum: c_int = 0;

    for i in 0..arr.count {
        // current = &arr.data[i], base = &arr.data[0].
        // weight = (current > base) ? (current - base) : 1.
        // Pointer subtraction yields the element index.
        let weight: c_int = if i > 0 { i } else { 1 };

        let current_value = arr.data[i as usize].value;
        let weighted = (current_value as f64) * (weight as f64) * 0.8;
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

    let values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        // param4 / 2 + 1: C signed division truncates toward zero.
        // Avoid INT_MIN / 2 overflow concerns: INT_MIN / 2 is well-defined.
        (param4 / 2).wrapping_add(1),
    ];

    let mut arr = ResultArray {
        data: [Result_::zero(); 10],
        count: 0,
    };
    init_result_array(&mut arr, &values, 8);

    let mut result: c_int = 0;

    for i in 0..4usize {
        result = result.wrapping_add(process_with_foreach(&mut arr, operations[i]));
    }

    result = result.wrapping_add(compute_weighted_sum(&mut arr));

    for i in 0..(arr.count - 1) {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
    }

    let final_scale = (result as f64) * 0.333;
    safe_double_to_int(final_scale)
}
