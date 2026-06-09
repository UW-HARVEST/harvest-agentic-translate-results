// Translated from c_src/src/lib.c
// Reads four integers from stdin (scanf-style) and prints arrayfunc(a, b, c, d).

use std::io::{self, Read, Write};

#[derive(Clone, Copy)]
struct Result {
    value: i32,
    scaled: f64,
    rank: i32,
}

struct ResultArray {
    data: [Result; 10],
    count: i32,
}

type OperationFunc = fn(i32, i32, i32, i32) -> i32;

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
    // Avoid panic on INT_MIN % -1 (which is undefined in C as well).
    if b == -1 {
        return 0;
    }
    a % b
}

fn safe_double_to_int(d: f64) -> i32 {
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    if d.is_nan() {
        return 0;
    }
    d as i32
}

fn compare_results_in_array(arr: &ResultArray, idx1: i32, idx2: i32) -> i32 {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }

    // ptr1 = &arr->data[idx1], ptr2 = &arr->data[idx2]
    // ptr1 < ptr2 iff idx1 < idx2 since data is contiguous.
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

    for i in 0..arr.count as usize {
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
        // current = &arr->data[i], base = &arr->data[0]
        // weight = (current > base) ? (int)(current - base) : 1
        let weight: i32 = if i > 0 { i as i32 } else { 1 };

        let weighted = arr.data[i].value as f64 * weight as f64 * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));
    }

    sum
}

fn arrayfunc(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
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
        data: [Result {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
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

fn read_all_stdin() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_to_string(&mut s);
    s
}

fn parse_four_ints(input: &str) -> Option<(i32, i32, i32, i32)> {
    // Mimic scanf("%d %d %d %d", ...): whitespace-separated integers,
    // skipping any leading whitespace, accepts integer with optional sign.
    let mut iter = input.split_ascii_whitespace();
    let a: i32 = iter.next()?.parse().ok()?;
    let b: i32 = iter.next()?.parse().ok()?;
    let c: i32 = iter.next()?.parse().ok()?;
    let d: i32 = iter.next()?.parse().ok()?;
    Some((a, b, c, d))
}

fn main() {
    let input = read_all_stdin();
    let (a, b, c, d) = match parse_four_ints(&input) {
        Some(t) => t,
        None => {
            // If input cannot be parsed, behave like scanf with uninitialized
            // ints — but we won't risk UB here. Default to zeros.
            (0, 0, 0, 0)
        }
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "{}", arrayfunc(a, b, c, d));
}
