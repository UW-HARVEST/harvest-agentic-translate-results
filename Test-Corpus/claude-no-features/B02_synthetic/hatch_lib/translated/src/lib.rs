// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust to produce byte-identical output for the same inputs.

use std::ffi::c_int;

// ----------------------------------------------------------------------------
// Module-level state mirroring the C `static` globals.
// ----------------------------------------------------------------------------

// SAFETY: This library mirrors C semantics exactly. The original C code uses
// non-thread-safe `static int` globals; we reproduce that behavior with
// `static mut` and unsafe access, accepting the same single-threaded
// assumptions as the C code.
static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

// ----------------------------------------------------------------------------
// Function pointer typedefs from C:
//   typedef int  (*operation_func)(int, int, int);
//   typedef void (*modifier_func)(int, int);
// ----------------------------------------------------------------------------

type OperationFunc = fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = fn(c_int, c_int);

// ----------------------------------------------------------------------------
// Helper functions (all internal to this crate; not exported).
// ----------------------------------------------------------------------------

fn increment_counter(value: c_int, _unused_param: c_int) {
    // global_counter += value;
    unsafe {
        GLOBAL_COUNTER = GLOBAL_COUNTER.wrapping_add(value);
    }
}

fn update_accumulator(value: c_int, _unused_param: c_int) {
    // global_accumulator = global_accumulator * 2 + value;
    unsafe {
        GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR.wrapping_mul(2).wrapping_add(value);
    }
}

fn apply_operation(op: OperationFunc, a: c_int, b: c_int, c: c_int) -> c_int {
    op(a, b, c)
}

fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(c)
}

fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    // (a - b) * c + global_counter
    let counter = unsafe { GLOBAL_COUNTER };
    a.wrapping_sub(b).wrapping_mul(c).wrapping_add(counter)
}

#[derive(Clone, Copy)]
#[repr(C)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64, // time_t; not read by hatch's return value, only written.
    name: [u8; 32],
}

fn shift_array_data(arr: &mut [c_int], size: usize, shift_by: usize) {
    if shift_by > 0 && shift_by < size {
        // memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int));
        arr.copy_within(shift_by..size, 0);
        // memset(arr + (size - shift_by), 0, shift_by * sizeof(int));
        for slot in &mut arr[(size - shift_by)..size] {
            *slot = 0;
        }
    }
}

fn process_pointer_data(value: c_int, multiplier: c_int) -> c_int {
    // value * multiplier + global_accumulator
    let acc = unsafe { GLOBAL_ACCUMULATOR };
    value.wrapping_mul(multiplier).wrapping_add(acc)
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    // Allocate a temporary array of `count` ints, fill, sum, free.
    let n = count as usize;
    let mut temp_array: Vec<c_int> = Vec::with_capacity(n);

    for i in 0..n {
        temp_array.push(base.wrapping_add((i as c_int).wrapping_mul(3)));
    }

    let mut sum: c_int = 0;
    for i in 0..n {
        sum = sum.wrapping_add(temp_array[i]);
    }

    // `temp_array` is dropped here (analogous to free).
    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    // The C code computes:
    //   time(&current_time);                              // current wall-clock
    //   reference_time = current_time - (seed * 3600);    // (seed*3600) at int
    //   diff = difftime(current_time, reference_time);    // == (long)(int)(seed*3600)
    //   return (int)(diff / 100) + seed;
    //
    // Because the difference cancels out the wall-clock value entirely, the
    // result depends only on `seed`. We mirror the C `int * int` multiplication
    // (which can wrap) using wrapping_mul so we match GCC's typical behavior.
    let int_product: c_int = seed.wrapping_mul(3600);
    let diff: f64 = int_product as f64;
    ((diff / 100.0) as c_int).wrapping_add(seed)
}

fn manipulate_records(records: &mut [DataRecord], num_records: usize, shift: usize) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        // memmove(records, records + shift, (num_records - shift) * sizeof(DataRecord));
        records.copy_within(shift..num_records, 0);
    }

    // for (int i = 0; i < num_records - shift; i++) { total += records[i].value; }
    // Note: this exactly mirrors the C; if shift == 0 this iterates the whole
    // array, and if shift >= num_records the C code performs a wraparound on
    // an unsigned subtraction in the loop bound; we preserve that by using
    // wrapping subtraction below.
    let limit = num_records.wrapping_sub(shift);
    for i in 0..limit {
        total = total.wrapping_add(records[i].value);
    }

    total
}

// ----------------------------------------------------------------------------
// Public C ABI entry point: int hatch(int, int, int, int)
// ----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    // modifier_func mod_func;
    let mut mod_func: ModifierFunc;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    // Suppress "unused assignment" warnings: mirror C's exact assignment pattern.
    let _ = mod_func;

    // operation_func op_func;
    let mut op_func: OperationFunc;

    op_func = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));

    let _ = op_func;

    // int *dynamic_data = malloc(10 * sizeof(int));
    let mut dynamic_data: Vec<c_int> = Vec::with_capacity(10);
    for i in 0..10 {
        dynamic_data.push(param1.wrapping_add(i as c_int));
    }

    // result += process_pointer_data(&dynamic_data[5], param2);
    result = result.wrapping_add(process_pointer_data(dynamic_data[5], param2));

    // shift_array_data(dynamic_data, 10, 3);
    shift_array_data(&mut dynamic_data, 10, 3);
    // result += dynamic_data[0];
    result = result.wrapping_add(dynamic_data[0]);

    // free(dynamic_data); -> drop happens at end of scope; explicit drop:
    drop(dynamic_data);

    // result += get_time_based_value(param3);
    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = malloc(5 * sizeof(DataRecord));
    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5 {
        let mut name = [0u8; 32];
        // snprintf(records[i].name, 32, "Record_%d", i);
        let s = format!("Record_{}", i);
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(31); // leave room for trailing NUL
        name[..copy_len].copy_from_slice(&bytes[..copy_len]);
        // remaining bytes already zeroed (NUL-terminator + padding)
        records.push(DataRecord {
            id: i as c_int,
            value: param4.wrapping_add((i as c_int).wrapping_mul(10)),
            timestamp: 0, // time(&records[i].timestamp); — value never read.
            name,
        });
    }

    // result += manipulate_records(records, 5, 2);
    result = result.wrapping_add(manipulate_records(&mut records, 5, 2));

    // free(records);
    drop(records);

    // result += compute_with_dynamic_memory(param1, 8);
    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    // result += global_counter + global_accumulator;
    let counter = unsafe { GLOBAL_COUNTER };
    let acc = unsafe { GLOBAL_ACCUMULATOR };
    result = result.wrapping_add(counter.wrapping_add(acc));

    result
}
