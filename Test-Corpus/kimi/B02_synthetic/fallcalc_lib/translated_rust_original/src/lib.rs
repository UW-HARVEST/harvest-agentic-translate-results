use std::os::raw::c_int;

const OCTAL_MASK_1: i32 = 0o777;
const OCTAL_MASK_2: i32 = 0o100;
const OCTAL_FLAG: i32 = 0o200;
const OCTAL_BASE: i32 = 0o10;

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

fn process_array_reverse(end: &[i32], count: usize) -> i32 {
    let mut sum = 0;
    let mut idx = end.len();

    for _ in 0..count {
        if idx > 0 {
            idx -= 1;
            sum += end[idx];
        }
    }

    sum
}

fn switch_fallthrough_calculator(value: i32, operation: i32) -> i32 {
    let mut result = value;

    match operation {
        0 => {
            result *= OCTAL_BASE;
            result += OCTAL_FLAG;
            result &= OCTAL_MASK_1;
        }
        1 => {
            result += OCTAL_FLAG;
            result &= OCTAL_MASK_1;
        }
        2 => {
            result &= OCTAL_MASK_1;
        }
        3 => {
            result *= 3;
            result += OCTAL_MASK_2;
        }
        4 => {
            result += OCTAL_MASK_2;
        }
        _ => {
            result = 0;
        }
    }

    result
}

fn allocate_and_compute(size: usize, multiplier: f64) -> i32 {
    let mut points: Vec<DataPoint> = Vec::with_capacity(size);

    for i in 0..size {
        points.push(DataPoint {
            value: (i as i32) * OCTAL_BASE,
            coefficient: (i as f64) * multiplier,
        });
    }

    let mut sum = 0.0;
    for point in &points {
        sum += (point.value as f64) * point.coefficient;
    }

    safe_double_to_int(sum)
}

fn foreach_sum(array: &[i32]) -> i32 {
    array.iter().sum()
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let base_value = param1 * OCTAL_MASK_2 + param2;

    let array_size = 5;
    let mut data_array: Vec<i32> = Vec::with_capacity(array_size);

    for i in 0..array_size {
        data_array.push(((i + 1) as i32) * OCTAL_BASE + param1);
    }

    let foreach_result = foreach_sum(&data_array);

    let reverse_sum = process_array_reverse(&data_array, array_size);

    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc = (param1 as f64) * 3.7 + (param2 as f64) * 2.3 - (param3 as f64) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute((param4 % 10 + 1) as usize, 1.5);

    let mut result = base_value + foreach_result + reverse_sum + switch_result + converted + alloc_result;

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    result &= OCTAL_MASK_1;

    result
}
