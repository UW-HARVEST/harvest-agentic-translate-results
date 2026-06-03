// Translated from c_src/src/lib.c

const OCTAL_MASK_1: i32 = 0o777; // 511
const OCTAL_MASK_2: i32 = 0o100; // 64
const OCTAL_FLAG: i32 = 0o200;   // 128
const OCTAL_BASE: i32 = 0o10;    // 8

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
    let mut sum: i32 = 0;
    let mut idx: isize = end_idx as isize;

    for _ in 0..count {
        sum = sum.wrapping_add(arr[idx as usize]);
        idx -= 1;
    }

    sum
}

fn switch_fallthrough_calculator(value: i32, operation: i32) -> i32 {
    let mut result = value;

    // The original C switch has fall-through; here we explicitly inline that
    // behavior for each case.
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
        return -1;
    }

    let size_us = size as usize;
    let mut points: Vec<DataPoint> = Vec::with_capacity(size_us);

    for i in 0..size {
        points.push(DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: (i as f64) * multiplier,
        });
    }

    let mut sum: f64 = 0.0;
    for p in &points {
        sum += (p.value as f64) * p.coefficient;
    }

    safe_double_to_int(sum)
}

fn foreach_sum(arr: &[i32], count: i32) -> i32 {
    let mut total: i32 = 0;
    let n = count as usize;
    for i in 0..n {
        total = total.wrapping_add(arr[i]);
    }
    total
}

// C's `%` operator is truncated division (sign follows dividend).
fn c_mod(a: i32, b: i32) -> i32 {
    a.wrapping_rem(b)
}

#[no_mangle]
pub extern "C" fn fallcalc(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: i32 = 5;
    let array_size_us = array_size as usize;
    let mut data_array: Vec<i32> = Vec::with_capacity(array_size_us);

    for i in 0..array_size {
        let val = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        data_array.push(val);
    }

    let foreach_result = foreach_sum(&data_array, array_size);

    let last_index = array_size_us - 1;
    let reverse_sum = process_array_reverse(&data_array, last_index, array_size);

    let switch_result = switch_fallthrough_calculator(param2, c_mod(param3, 5));

    let floating_calc =
        (param1 as f64) * 3.7 + (param2 as f64) * 2.3 - (param3 as f64) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute(c_mod(param4, 10) + 1, 1.5);

    let mut result = base_value
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
