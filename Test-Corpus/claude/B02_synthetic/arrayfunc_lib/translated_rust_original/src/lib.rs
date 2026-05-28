// Translation of c_src/src/lib.c to Rust.
//
// This is a faithful translation that reproduces byte-identical output
// for the same inputs.

use std::ffi::c_int;

type OperationFunc = fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Copy, Clone)]
#[repr(C)]
struct Result {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

#[repr(C)]
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
    // C `%` truncates toward zero; Rust `%` does the same on integers.
    // Avoid INT_MIN % -1 overflow as in the original (which would invoke UB in C).
    if b == -1 {
        return 0;
    }
    a % b
}

fn safe_double_to_int(d: f64) -> c_int {
    if d >= (i32::MAX as f64) {
        return i32::MAX;
    }
    if d <= (i32::MIN as f64) {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    d as c_int
}

fn compare_results_in_array(arr: &ResultArray, idx1: c_int, idx2: c_int) -> c_int {
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }

    let ptr1: *const Result = &arr.data[idx1 as usize];
    let ptr2: *const Result = &arr.data[idx2 as usize];

    if ptr1 < ptr2 {
        return -1;
    } else if ptr1 > ptr2 {
        return 1;
    }
    0
}

fn init_result_array(arr: &mut ResultArray, values: &[c_int], count: c_int) {
    arr.count = if count < 10 { count } else { 10 };

    for i in 0..arr.count as usize {
        arr.data[i] = Result {
            value: values[i],
            scaled: (values[i] as f64) * 1.5,
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

        let temp = (result as f64) * 0.75;
        item.scaled = temp;
        item.value = safe_double_to_int(temp);
    }

    total
}

fn compute_weighted_sum(arr: &mut ResultArray) -> c_int {
    let mut sum: c_int = 0;

    let count = arr.count as usize;
    // Capture the base pointer once so the comparison reflects the original
    // allocation address, mirroring the C code's behavior.
    let base_ptr: *const Result = &arr.data[0];

    for i in 0..count {
        let current_ptr: *const Result = &arr.data[i];

        let weight: c_int = if current_ptr > base_ptr {
            // Pointer difference is in units of Result.
            unsafe { current_ptr.offset_from(base_ptr) as c_int }
        } else {
            1
        };

        let value = arr.data[i].value;
        let weighted = (value as f64) * (weight as f64) * 0.8;
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

    let mut result: c_int = 0;

    for i in 0..4 {
        result = result.wrapping_add(process_with_foreach(&mut arr, operations[i]));
    }

    result = result.wrapping_add(compute_weighted_sum(&mut arr));

    for i in 0..(arr.count - 1) {
        let cmp = compare_results_in_array(&arr, i, i + 1);
        result = result.wrapping_add(cmp);
    }

    let final_scale = (result as f64) * 0.333;
    result = safe_double_to_int(final_scale);

    result
}
