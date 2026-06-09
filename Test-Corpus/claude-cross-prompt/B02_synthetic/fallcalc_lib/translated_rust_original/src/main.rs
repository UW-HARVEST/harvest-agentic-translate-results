// Translated from C to Rust. Reproduces behavior of fallcalc().

use std::io::{self, Read};

const OCTAL_MASK_1: i32 = 0o777;
const OCTAL_MASK_2: i32 = 0o100;
const OCTAL_FLAG: i32 = 0o200;
const OCTAL_BASE: i32 = 0o10;

#[derive(Clone, Copy)]
struct DataPoint {
    value: i32,
    coefficient: f64,
}

fn safe_double_to_int(d: f64) -> i32 {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { i32::MAX } else { i32::MIN };
    }

    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }

    d as i32
}

fn process_array_reverse(arr: &[i32], end_idx: usize, count: i32) -> i32 {
    // Mirrors: int *ptr = end; for (i=0; i<count; i++) sum += *ptr; ptr--;
    let mut sum: i32 = 0;
    let mut idx: isize = end_idx as isize;
    for _ in 0..count {
        // Use wrapping arithmetic to mirror C's signed int addition behavior.
        sum = sum.wrapping_add(arr[idx as usize]);
        idx -= 1;
    }
    sum
}

fn switch_fallthrough_calculator(value: i32, operation: i32) -> i32 {
    let mut result: i32 = value;

    match operation {
        0 => {
            result = result.wrapping_mul(OCTAL_BASE);
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        1 => {
            result = result.wrapping_add(OCTAL_FLAG);
            result &= OCTAL_MASK_1;
        }
        2 => {
            result &= OCTAL_MASK_1;
        }
        3 => {
            result = result.wrapping_mul(3);
            result = result.wrapping_add(OCTAL_MASK_2);
        }
        4 => {
            result = result.wrapping_add(OCTAL_MASK_2);
        }
        _ => {
            result = 0;
        }
    }

    result
}

fn allocate_and_compute(size: i32, multiplier: f64) -> i32 {
    if size < 0 {
        // Mirror malloc returning NULL behavior for invalid sizes.
        return -1;
    }

    let size_usz = size as usize;
    let mut points: Vec<DataPoint> = vec![
        DataPoint {
            value: 0,
            coefficient: 0.0
        };
        size_usz
    ];

    for i in 0..size {
        let idx = i as usize;
        points[idx].value = i.wrapping_mul(OCTAL_BASE);
        points[idx].coefficient = (i as f64) * multiplier;
    }

    let mut sum: f64 = 0.0;
    for i in 0..size_usz {
        sum += (points[i].value as f64) * points[i].coefficient;
    }

    safe_double_to_int(sum)
}

fn foreach_sum(array: &[i32], count: i32) -> i32 {
    // Mirrors the FOREACH macro behavior.
    let mut total: i32 = 0;
    let mut idx: i32 = 0;
    let size = count;
    while idx < size {
        let element = array[idx as usize];
        total = total.wrapping_add(element);
        idx += 1;
    }
    total
}

fn fallcalc(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32;

    let base_value: i32 = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: i32 = 5;
    let mut data_array: Vec<i32> = vec![0; array_size as usize];

    for i in 0..array_size {
        data_array[i as usize] = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
    }

    let foreach_result = foreach_sum(&data_array, array_size);

    let last_index = (array_size - 1) as usize;
    let reverse_sum = process_array_reverse(&data_array, last_index, array_size);

    // C's `%` for negative dividends yields a negative remainder (C99+),
    // matching Rust's `%` for primitive integers.
    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc: f64 =
        (param1 as f64) * 3.7 + (param2 as f64) * 2.3 - (param3 as f64) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute(param4 % 10 + 1, 1.5);

    result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    result &= OCTAL_MASK_1;

    result
}

fn main() {
    // Read all of stdin and tokenize by whitespace, matching scanf("%d", ...) behavior.
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        std::process::exit(1);
    }

    let mut tokens = input.split_ascii_whitespace();

    let mut read_int = || -> Option<i32> {
        let tok = tokens.next()?;
        tok.parse::<i32>().ok()
    };

    let a = match read_int() {
        Some(v) => v,
        None => return,
    };
    let b = match read_int() {
        Some(v) => v,
        None => return,
    };
    let c = match read_int() {
        Some(v) => v,
        None => return,
    };
    let d = match read_int() {
        Some(v) => v,
        None => return,
    };

    let result = fallcalc(a, b, c, d);
    println!("{}", result);
}
