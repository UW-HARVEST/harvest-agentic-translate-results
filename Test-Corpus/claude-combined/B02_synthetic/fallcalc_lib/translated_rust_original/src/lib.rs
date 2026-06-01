// Rust translation of c_src/src/lib.c.
//
// Goals:
//   * Byte-identical numerical output to the C implementation for all inputs.
//   * Exposed symbol names match the C library (no Rust mangling on public
//     functions, since C doesn't define a renaming preprocessor macro for any
//     of them).
//   * The function signatures in the C ABI mirror the original C signatures
//     so callers cannot tell the implementations apart.

use std::ffi::c_int;
use std::os::raw::c_double;

// Octal constants from the C header — kept in octal for readability.
const OCTAL_MASK_1: c_int = 0o777;
const OCTAL_MASK_2: c_int = 0o100;
const OCTAL_FLAG: c_int = 0o200;
const OCTAL_BASE: c_int = 0o10;

#[repr(C)]
#[derive(Copy, Clone)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }

    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }

    d as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = end;

    for _ in 0..count {
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = unsafe { ptr.offset(-1) };
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    // The C function uses fall-through `case` labels:
    //
    //   case 0: result *= OCTAL_BASE;     // fall through
    //   case 1: result += OCTAL_FLAG;     // fall through
    //   case 2: result &= OCTAL_MASK_1;
    //           break;
    //   case 3: result *= 3;              // fall through
    //   case 4: result += OCTAL_MASK_2;
    //           break;
    //   default: result = 0;
    //
    // Each match arm below replicates the cumulative effect of the C cascade.
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

#[unsafe(no_mangle)]
pub extern "C" fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    // The C version does `malloc(size * sizeof(DataPoint))` and returns -1
    // on allocation failure. With negative `size`, that multiplication wraps
    // and likely fails to allocate, so we conservatively return -1 in that
    // case to mirror the failure path.
    if size < 0 {
        return -1;
    }

    let mut points: Vec<DataPoint> = Vec::with_capacity(size as usize);
    for i in 0..size {
        points.push(DataPoint {
            value: i.wrapping_mul(OCTAL_BASE),
            coefficient: (i as c_double) * multiplier,
        });
    }

    let mut sum: c_double = 0.0;
    for i in 0..(size as usize) {
        sum += (points[i].value as c_double) * points[i].coefficient;
    }

    safe_double_to_int(sum)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;
    for i in 0..count {
        total = total.wrapping_add(unsafe { *array.offset(i as isize) });
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int;

    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let mut data_array: Vec<c_int> = Vec::with_capacity(array_size as usize);
    for i in 0..array_size {
        data_array.push((i + 1).wrapping_mul(OCTAL_BASE).wrapping_add(param1));
    }

    let foreach_result = unsafe { foreach_sum(data_array.as_mut_ptr(), array_size) };

    let last_element = unsafe { data_array.as_mut_ptr().offset((array_size - 1) as isize) };
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };

    // Rust's `%` on i32 is the C-style truncating remainder, identical to
    // C's `%` for signed `int` on two's-complement platforms.
    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc =
        (param1 as c_double) * 3.7 + (param2 as c_double) * 2.3 - (param3 as c_double) * 0.5;
    let converted = safe_double_to_int(floating_calc);

    let alloc_result = allocate_and_compute((param4 % 10).wrapping_add(1), 1.5);

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
