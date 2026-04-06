use std::os::raw::c_int;

const OCTAL_MASK_1: c_int = 0o777; // 511
const OCTAL_MASK_2: c_int = 0o100; // 64
const OCTAL_FLAG: c_int = 0o200; // 128
const OCTAL_BASE: c_int = 0o10; // 8

fn safe_double_to_int(d: f64) -> c_int {
    if d.is_nan() {
        return 0;
    }
    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }
    if d >= c_int::MAX as f64 {
        return c_int::MAX;
    }
    if d <= c_int::MIN as f64 {
        return c_int::MIN;
    }
    d as c_int
}

fn process_array_reverse(data: &[c_int]) -> c_int {
    // C version: starts at end pointer, walks backward count elements
    let mut sum: c_int = 0;
    for &v in data.iter().rev() {
        sum = sum.wrapping_add(v);
    }
    sum
}

fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result = value;
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

fn allocate_and_compute(size: c_int, multiplier: f64) -> c_int {
    // C code does malloc(size * sizeof(DataPoint)). If size <= 0, the C
    // multiplication wraps via unsigned size_t and malloc likely fails → -1.
    if size <= 0 {
        return -1;
    }
    let mut sum = 0.0_f64;
    for i in 0..size {
        let value = i.wrapping_mul(OCTAL_BASE);
        let coefficient = i as f64 * multiplier;
        sum += value as f64 * coefficient;
    }
    safe_double_to_int(sum)
}

fn foreach_sum(array: &[c_int]) -> c_int {
    let mut total: c_int = 0;
    for &element in array {
        total = total.wrapping_add(element);
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size = 5;
    let mut data_array = Vec::with_capacity(array_size);
    for i in 0..array_size as c_int {
        data_array.push((i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1));
    }

    let foreach_result = foreach_sum(&data_array);
    let reverse_sum = process_array_reverse(&data_array);

    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc = param1 as f64 * 3.7 + param2 as f64 * 2.3 - param3 as f64 * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_size = param4 % 10 + 1;
    let alloc_result = allocate_and_compute(alloc_size, 1.5);

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
