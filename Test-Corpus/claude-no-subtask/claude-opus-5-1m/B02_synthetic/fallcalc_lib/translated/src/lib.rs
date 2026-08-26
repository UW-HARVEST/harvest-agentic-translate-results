// Translated from c_src/src/lib.c
// Library exposing `fallcalc` with the same C ABI and byte-identical behavior.

use std::os::raw::c_int;

const OCTAL_MASK_1: c_int = 0o777; // 511
const OCTAL_MASK_2: c_int = 0o100; // 64
const OCTAL_FLAG: c_int = 0o200;   // 128
const OCTAL_BASE: c_int = 0o10;    // 8

#[derive(Clone, Copy)]
struct DataPoint {
    value: c_int,
    coefficient: f64,
}

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

/// Mirrors the C function which walks backwards from `end` reading `count` ints.
fn process_array_reverse(end: *const c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;

    let mut i: c_int = 0;
    while i < count {
        unsafe {
            // Read element at offset -i from `end`.
            let v = *end.offset(-(i as isize));
            sum = sum.wrapping_add(v);
        }
        i += 1;
    }

    sum
}

fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result: c_int = value;

    // Replicate C-style fallthrough exactly.
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
    // In C, when size is negative, `size * sizeof(DataPoint)` is converted to
    // size_t which yields a huge value, causing malloc to return NULL and the
    // function to return -1. Reproduce that early-out here.
    if size < 0 {
        return -1;
    }

    let mut points: Vec<DataPoint> = Vec::with_capacity(size as usize);
    for i in 0..size {
        points.push(DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: (i as f64) * multiplier,
        });
    }

    let mut sum: f64 = 0.0;
    for i in 0..size {
        let p = points[i as usize];
        sum += (p.value as f64) * p.coefficient;
    }

    safe_double_to_int(sum)
}

fn foreach_sum(array: *const c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;
    let mut idx: c_int = 0;
    while idx < count {
        let element = unsafe { *array.offset(idx as isize) };
        total = total.wrapping_add(element);
        idx += 1;
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int;

    let base_value: c_int = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    // Allocation will not fail with a small fixed size; mirror the C code by
    // owning the memory and freeing at end of scope.
    let mut data_array: Vec<c_int> = Vec::with_capacity(array_size as usize);
    // Note: the C `if (data_array == NULL) return -1;` path is unreachable in
    // practice for a 5-element allocation, so we don't simulate it.

    for i in 0..array_size {
        let v = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
        data_array.push(v);
    }

    let foreach_result = foreach_sum(data_array.as_ptr(), array_size);

    let last_element = unsafe { data_array.as_ptr().offset((array_size - 1) as isize) };
    let reverse_sum = process_array_reverse(last_element, array_size);

    let switch_result = switch_fallthrough_calculator(param2, param3.rem_euclid_c(5));

    let floating_calc: f64 =
        (param1 as f64) * 3.7 + (param2 as f64) * 2.3 - (param3 as f64) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute(param4.rem_euclid_c(10).wrapping_add(1), 1.5);

    result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    // `data_array` dropped here (equivalent to free()).
    drop(data_array);

    result &= OCTAL_MASK_1;

    result
}

// Helper trait to provide C-style `%` semantics (truncated remainder, sign of
// dividend) for c_int. Rust's `%` operator already implements this for signed
// integers, so this simply forwards.
trait CStyleRem {
    fn rem_euclid_c(self, rhs: c_int) -> c_int;
}

impl CStyleRem for c_int {
    #[inline]
    fn rem_euclid_c(self, rhs: c_int) -> c_int {
        // Rust's `%` for signed ints matches C's truncated remainder semantics.
        self % rhs
    }
}
