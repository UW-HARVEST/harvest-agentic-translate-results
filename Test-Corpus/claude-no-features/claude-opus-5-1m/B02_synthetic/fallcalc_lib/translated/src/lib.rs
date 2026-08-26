// Rust translation of c_src/src/lib.c
// Preserves exact behavior of the original C code.

use std::os::raw::c_int;

const OCTAL_MASK_1: c_int = 0o777;
const OCTAL_MASK_2: c_int = 0o100;
const OCTAL_FLAG: c_int = 0o200;
const OCTAL_BASE: c_int = 0o10;

#[repr(C)]
#[derive(Copy, Clone)]
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

    // Truncates toward zero, matching C's (int)d cast for values within range.
    d as c_int
}

/// Mirrors the C `process_array_reverse(int *end, int count)`. The caller
/// passes a pointer to the last element of an array and we walk backwards.
unsafe fn process_array_reverse(end: *const c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = end;

    for _ in 0..count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = unsafe { ptr.offset(-1) };
    }

    sum
}

fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    let mut result = value;

    // The original C code uses fall-through between cases. We replicate it
    // explicitly with sequential conditional blocks.
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
    if size < 0 {
        // Match malloc() returning NULL semantics for invalid sizes.
        return -1;
    }

    let size_usz = size as usize;
    let mut points: Vec<DataPoint> = Vec::with_capacity(size_usz);

    for i in 0..size {
        points.push(DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: (i as f64) * multiplier,
        });
    }

    let mut sum: f64 = 0.0;
    for i in 0..size_usz {
        sum += (points[i].value as f64) * points[i].coefficient;
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
    let mut result: c_int;

    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let array_size_usz = array_size as usize;
    let mut data_array: Vec<c_int> = vec![0; array_size_usz];

    for i in 0..array_size {
        data_array[i as usize] = (i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1);
    }

    let foreach_result = foreach_sum(&data_array);

    // Pointer to the last element, then walk backward, just like the C code.
    let last_element = unsafe { data_array.as_ptr().add((array_size - 1) as usize) };
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };

    let switch_result = switch_fallthrough_calculator(param2, param3.rem_euclid_c(5));

    let floating_calc =
        (param1 as f64) * 3.7 + (param2 as f64) * 2.3 - (param3 as f64) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute(param4.rem_euclid_c(10) + 1, 1.5);

    result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    // `data_array` is dropped here, matching the `free(data_array)` call.
    drop(data_array);

    result &= OCTAL_MASK_1;

    result
}

/// Helper trait to perform C-style `%` (truncated remainder) on `c_int`.
/// Rust's `%` operator on i32 already matches C semantics (truncation toward
/// zero), but we add a named method to make the intent explicit and to avoid
/// any future ambiguity. We deliberately call this `rem_euclid_c` (despite the
/// name) to mean "C-style remainder", not Euclidean.
trait CRem {
    fn rem_euclid_c(self, rhs: Self) -> Self;
}

impl CRem for c_int {
    #[inline]
    fn rem_euclid_c(self, rhs: Self) -> Self {
        // i32 `%` in Rust matches C99 `%` for non-overflowing inputs.
        // Use wrapping_rem to also match C's undefined-behavior corner case
        // (INT_MIN % -1) without panicking.
        self.wrapping_rem(rhs)
    }
}
