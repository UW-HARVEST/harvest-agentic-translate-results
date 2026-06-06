// Translated from c_src/src/lib.c
// Preserves byte-identical output for the same inputs.

use std::ffi::c_int;
use std::sync::Mutex;

// Use Mutex-wrapped statics to mirror C's mutable globals while remaining safe in Rust.
static GLOBAL_COUNTER: Mutex<c_int> = Mutex::new(0);
static GLOBAL_ACCUMULATOR: Mutex<c_int> = Mutex::new(0);

#[repr(C)]
#[derive(Clone, Copy)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64, // time_t
    name: [u8; 32],
}

fn increment_counter(value: c_int, _unused_param: c_int) {
    let mut counter = GLOBAL_COUNTER.lock().unwrap();
    *counter = counter.wrapping_add(value);
}

fn update_accumulator(value: c_int, _unused_param: c_int) {
    let mut acc = GLOBAL_ACCUMULATOR.lock().unwrap();
    *acc = acc.wrapping_mul(2).wrapping_add(value);
}

type OperationFunc = fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = fn(c_int, c_int);

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
    let counter = *GLOBAL_COUNTER.lock().unwrap();
    a.wrapping_sub(b).wrapping_mul(c).wrapping_add(counter)
}

fn shift_array_data(arr: &mut [c_int], size: usize, shift_by: usize) {
    if shift_by > 0 && shift_by < size {
        // memmove arr, arr + shift_by, (size - shift_by)
        arr.copy_within(shift_by..size, 0);
        // memset arr + (size - shift_by), 0, shift_by
        for i in (size - shift_by)..size {
            arr[i] = 0;
        }
    }
}

fn process_pointer_data(value: c_int, multiplier: c_int) -> c_int {
    let acc = *GLOBAL_ACCUMULATOR.lock().unwrap();
    value.wrapping_mul(multiplier).wrapping_add(acc)
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let count_usize = count as usize;
    let mut temp_array: Vec<c_int> = vec![0; count_usize];

    for i in 0..count_usize {
        temp_array[i] = base.wrapping_add((i as c_int).wrapping_mul(3));
    }

    let mut sum: c_int = 0;
    for i in 0..count_usize {
        sum = sum.wrapping_add(temp_array[i]);
    }

    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    // The C code computes:
    //   current_time = time(...)
    //   reference_time = current_time - (seed * 3600)
    //   diff = difftime(current_time, reference_time)
    //   return (int)(diff / 100) + seed
    //
    // Since reference_time is derived from current_time, the time(...) call
    // cancels out and the result is purely deterministic:
    //   diff = seed * 3600 (as a double)
    //   (int)(diff / 100) + seed = seed * 36 + seed = seed * 37
    //
    // We mirror that arithmetic to produce identical output.
    let diff: f64 = (seed as f64) * 3600.0;
    (diff / 100.0) as c_int + seed
}

fn manipulate_records(records: &mut [DataRecord], num_records: c_int, shift: c_int) -> c_int {
    let mut total: c_int = 0;

    let num = num_records as usize;
    if shift > 0 && shift < num_records {
        let s = shift as usize;
        records.copy_within(s..num, 0);
    }

    let limit = num_records - shift; // matches C's (i < num_records - shift)
    let mut i: c_int = 0;
    while i < limit {
        total = total.wrapping_add(records[i as usize].value);
        i += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut mod_func: ModifierFunc;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    let mut op_func: OperationFunc;

    op_func = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));

    // Allocate a buffer of 10 ints (matches malloc(10 * sizeof(int)))
    let mut dynamic_data: Vec<c_int> = vec![0; 10];
    for i in 0..10 {
        dynamic_data[i] = param1.wrapping_add(i as c_int);
    }

    // process_pointer_data(&dynamic_data[5], param2)
    let val = dynamic_data[5];
    result = result.wrapping_add(process_pointer_data(val, param2));

    shift_array_data(&mut dynamic_data, 10, 3);
    result = result.wrapping_add(dynamic_data[0]);

    drop(dynamic_data);

    result = result.wrapping_add(get_time_based_value(param3));

    // Allocate 5 DataRecords
    let mut records: Vec<DataRecord> = vec![
        DataRecord {
            id: 0,
            value: 0,
            timestamp: 0,
            name: [0u8; 32],
        };
        5
    ];

    for i in 0..5 {
        records[i].id = i as c_int;
        records[i].value = param4.wrapping_add((i as c_int).wrapping_mul(10));
        // time(&records[i].timestamp); -- timestamp is never read; leave as 0.
        // snprintf(records[i].name, 32, "Record_%d", i);
        let name_str = format!("Record_{}", i);
        let bytes = name_str.as_bytes();
        let n = bytes.len().min(31);
        records[i].name[..n].copy_from_slice(&bytes[..n]);
        records[i].name[n] = 0;
    }

    result = result.wrapping_add(manipulate_records(&mut records, 5, 2));

    drop(records);

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    let counter = *GLOBAL_COUNTER.lock().unwrap();
    let acc = *GLOBAL_ACCUMULATOR.lock().unwrap();
    result = result.wrapping_add(counter.wrapping_add(acc));

    result
}
