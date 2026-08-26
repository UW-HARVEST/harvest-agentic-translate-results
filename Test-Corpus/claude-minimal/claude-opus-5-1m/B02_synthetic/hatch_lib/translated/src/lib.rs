// Rust translation of c_src/src/lib.c
//
// Original copyright:
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
// (See c_src/src/lib.c for full license text.)

use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// Global mutable state, equivalent to the C `static int` globals.
static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
static GLOBAL_ACCUMULATOR: AtomicI32 = AtomicI32::new(0);

// Function-pointer type aliases mirroring the C typedefs.
type OperationFunc = fn(i32, i32, i32) -> i32;
type ModifierFunc = fn(i32, i32);

fn increment_counter(value: i32, _unused_param: i32) {
    // global_counter += value;
    let prev = GLOBAL_COUNTER.load(Ordering::Relaxed);
    GLOBAL_COUNTER.store(prev.wrapping_add(value), Ordering::Relaxed);
}

fn update_accumulator(value: i32, _unused_param: i32) {
    // global_accumulator = global_accumulator * 2 + value;
    let prev = GLOBAL_ACCUMULATOR.load(Ordering::Relaxed);
    GLOBAL_ACCUMULATOR.store(prev.wrapping_mul(2).wrapping_add(value), Ordering::Relaxed);
}

fn apply_operation(op: OperationFunc, a: i32, b: i32, c: i32) -> i32 {
    op(a, b, c)
}

fn add_three(a: i32, b: i32, c: i32) -> i32 {
    a.wrapping_add(b).wrapping_add(c)
}

fn multiply_add(a: i32, b: i32, c: i32) -> i32 {
    a.wrapping_mul(b).wrapping_add(c)
}

fn complex_calc(a: i32, b: i32, c: i32) -> i32 {
    // (a - b) * c + global_counter
    a.wrapping_sub(b)
        .wrapping_mul(c)
        .wrapping_add(GLOBAL_COUNTER.load(Ordering::Relaxed))
}

#[derive(Clone)]
struct DataRecord {
    #[allow(dead_code)]
    id: i32,
    value: i32,
    #[allow(dead_code)]
    timestamp: i64,
    #[allow(dead_code)]
    name: [u8; 32],
}

impl Default for DataRecord {
    fn default() -> Self {
        DataRecord {
            id: 0,
            value: 0,
            timestamp: 0,
            name: [0u8; 32],
        }
    }
}

fn shift_array_data(arr: &mut [i32], shift_by: usize) {
    let size = arr.len();
    if shift_by > 0 && shift_by < size {
        // memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int));
        arr.copy_within(shift_by..size, 0);
        // memset(arr + (size - shift_by), 0, shift_by * sizeof(int));
        for slot in arr.iter_mut().skip(size - shift_by) {
            *slot = 0;
        }
    }
}

fn process_pointer_data(value: i32, multiplier: i32) -> i32 {
    // *ptr * multiplier + global_accumulator
    value
        .wrapping_mul(multiplier)
        .wrapping_add(GLOBAL_ACCUMULATOR.load(Ordering::Relaxed))
}

fn compute_with_dynamic_memory(base: i32, count: i32) -> i32 {
    let count_usz = count as usize;
    let mut temp_array: Vec<i32> = Vec::with_capacity(count_usz);
    for i in 0..count {
        temp_array.push(base.wrapping_add(i.wrapping_mul(3)));
    }

    let mut sum: i32 = 0;
    for i in 0..count_usz {
        sum = sum.wrapping_add(temp_array[i]);
    }

    sum
}

fn current_unix_time_secs() -> i64 {
    // Mirrors C's time(NULL) which returns seconds since the Unix epoch.
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

fn get_time_based_value(seed: i32) -> i32 {
    // time_t current_time; time(&current_time);
    let current_time = current_unix_time_secs();
    // reference_time = current_time - (seed * 3600);
    let reference_time = current_time.wrapping_sub((seed as i64).wrapping_mul(3600));
    // double diff = difftime(current_time, reference_time);
    let diff: f64 = (current_time - reference_time) as f64;
    // (int)(diff / 100) + seed
    ((diff / 100.0) as i32).wrapping_add(seed)
}

fn manipulate_records(records: &mut [DataRecord], shift: usize) -> i32 {
    let num_records = records.len();
    let mut total: i32 = 0;

    if shift > 0 && shift < num_records {
        // memmove(records, records + shift,
        //         (num_records - shift) * sizeof(DataRecord));
        for i in 0..(num_records - shift) {
            records[i] = records[i + shift].clone();
        }
    }

    // for (int i = 0; i < num_records - shift; i++) total += records[i].value;
    // Note: in the C source, when shift >= num_records this becomes a large
    // unsigned underflow in practice; we mirror the well-formed case.
    if shift <= num_records {
        for i in 0..(num_records - shift) {
            total = total.wrapping_add(records[i].value);
        }
    }

    total
}

fn write_record_name(buf: &mut [u8; 32], idx: i32) {
    // Equivalent to snprintf(buf, 32, "Record_%d", idx)
    let s = format!("Record_{}", idx);
    let bytes = s.as_bytes();
    let n = std::cmp::min(bytes.len(), 31);
    for i in 0..32 {
        buf[i] = 0;
    }
    for i in 0..n {
        buf[i] = bytes[i];
    }
}

#[no_mangle]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: i32 = 0;

    // modifier_func mod_func;
    let mut mod_func: ModifierFunc;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    // operation_func op_func;
    let mut op_func: OperationFunc;

    op_func = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));

    // int *dynamic_data = malloc(10 * sizeof(int));
    let mut dynamic_data: Vec<i32> = Vec::with_capacity(10);
    for i in 0..10 {
        dynamic_data.push(param1.wrapping_add(i));
    }

    // process_pointer_data(&dynamic_data[5], param2);
    result = result.wrapping_add(process_pointer_data(dynamic_data[5], param2));

    shift_array_data(&mut dynamic_data, 3);
    result = result.wrapping_add(dynamic_data[0]);

    drop(dynamic_data); // mirror free(dynamic_data)

    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = malloc(5 * sizeof(DataRecord));
    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5 {
        let mut rec = DataRecord::default();
        rec.id = i;
        rec.value = param4.wrapping_add(i.wrapping_mul(10));
        rec.timestamp = current_unix_time_secs();
        write_record_name(&mut rec.name, i);
        records.push(rec);
    }

    result = result.wrapping_add(manipulate_records(&mut records, 2));

    drop(records); // mirror free(records)

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    result = result
        .wrapping_add(GLOBAL_COUNTER.load(Ordering::Relaxed))
        .wrapping_add(GLOBAL_ACCUMULATOR.load(Ordering::Relaxed));

    result as c_int
}
