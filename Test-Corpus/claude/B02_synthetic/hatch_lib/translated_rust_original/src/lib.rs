// Translation of c_src/src/lib.c to Rust.
// Preserves the original behavior of `hatch` (and its helpers).
//
// Note: The original C uses two file-static globals (`global_counter` and
// `global_accumulator`) which are mutated by side-effecting helper functions.
// We replicate that with `static mut`. The C code is not thread-safe either,
// so we mirror that exactly.

use std::ffi::c_int;
use std::os::raw::c_char;
use std::time::{SystemTime, UNIX_EPOCH};

// ---- file-static globals -----------------------------------------------

static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

// ---- function-pointer typedefs (replicated as fn types) ----------------

type OperationFunc = fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = fn(c_int, c_int);

// ---- modifier functions ------------------------------------------------

fn increment_counter(value: c_int, _unused_param: c_int) {
    unsafe {
        GLOBAL_COUNTER = GLOBAL_COUNTER.wrapping_add(value);
    }
}

fn update_accumulator(value: c_int, _unused_param: c_int) {
    unsafe {
        GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR
            .wrapping_mul(2)
            .wrapping_add(value);
    }
}

// ---- operation functions -----------------------------------------------

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
    let counter = unsafe { GLOBAL_COUNTER };
    a.wrapping_sub(b).wrapping_mul(c).wrapping_add(counter)
}

// ---- DataRecord (matches the layout used in C) -------------------------
//
// In C this is:
//   typedef struct {
//       int id;
//       int value;
//       time_t timestamp;
//       char name[32];
//   } DataRecord;
//
// The struct layout is irrelevant to the externally-visible behavior here
// (records are allocated, written, then read on the same process), but we
// keep a faithful representation.

#[repr(C)]
#[derive(Clone, Copy)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64, // time_t
    name: [c_char; 32],
}

// ---- helpers -----------------------------------------------------------

fn shift_array_data(arr: &mut [c_int], shift_by: usize) {
    let size = arr.len();
    if shift_by > 0 && shift_by < size {
        // memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int));
        arr.copy_within(shift_by..size, 0);
        // memset(arr + (size - shift_by), 0, shift_by * sizeof(int));
        for i in (size - shift_by)..size {
            arr[i] = 0;
        }
    }
}

fn process_pointer_data(value: c_int, multiplier: c_int) -> c_int {
    let acc = unsafe { GLOBAL_ACCUMULATOR };
    value.wrapping_mul(multiplier).wrapping_add(acc)
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let count_usize = count as usize;
    let mut temp_array: Vec<c_int> = Vec::with_capacity(count_usize);

    for i in 0..count {
        temp_array.push(base.wrapping_add(i.wrapping_mul(3)));
    }

    let mut sum: c_int = 0;
    for i in 0..count_usize {
        sum = sum.wrapping_add(temp_array[i]);
    }

    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    // In C:
    //   time(&current_time);
    //   reference_time = current_time - (seed * 3600);
    //   diff = difftime(current_time, reference_time);  // == seed * 3600.0
    //   return (int)(diff / 100) + seed;                // == seed * 36 + seed
    //
    // The result is purely a function of `seed`: seed * 37 (with C's int
    // overflow semantics). We still call SystemTime::now() to mirror the
    // side-effect of reading the clock, even though the result does not
    // depend on it.
    let _ = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let diff_div_100 = seed.wrapping_mul(36);
    diff_div_100.wrapping_add(seed)
}

fn manipulate_records(records: &mut [DataRecord], shift: c_int) -> c_int {
    let num_records = records.len() as c_int;
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        let shift_usize = shift as usize;
        let n = records.len();
        // memmove(records, records + shift, (num_records - shift) * sizeof(DataRecord));
        records.copy_within(shift_usize..n, 0);
    }

    // Note: the C loop bound is `num_records - shift`; this is signed and may
    // be negative if shift > num_records. We mirror this exactly.
    let bound = num_records.wrapping_sub(shift);
    let mut i: c_int = 0;
    while i < bound {
        total = total.wrapping_add(records[i as usize].value);
        i = i.wrapping_add(1);
    }

    total
}

// ---- public entry point ------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn hatch(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    // mod_func = increment_counter; mod_func(param1, 999);
    let mut mod_func: ModifierFunc = increment_counter;
    mod_func(param1, 999);

    // mod_func = update_accumulator; mod_func(param2, 888);
    mod_func = update_accumulator;
    mod_func(param2, 888);

    // op_func = add_three; result += apply_operation(op_func, param1, param2, param3);
    let mut op_func: OperationFunc = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    // op_func = multiply_add; result += apply_operation(op_func, param2, param3, param4);
    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    // op_func = complex_calc; result += apply_operation(op_func, param1, param3, param4);
    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));

    // int *dynamic_data = malloc(10 * sizeof(int));
    let mut dynamic_data: Vec<c_int> = Vec::with_capacity(10);
    for i in 0..10 {
        dynamic_data.push(param1.wrapping_add(i));
    }

    // result += process_pointer_data(&dynamic_data[5], param2);
    result = result.wrapping_add(process_pointer_data(dynamic_data[5], param2));

    // shift_array_data(dynamic_data, 10, 3);
    shift_array_data(&mut dynamic_data, 3);

    // result += dynamic_data[0];
    result = result.wrapping_add(dynamic_data[0]);

    // free(dynamic_data); -- handled by Vec drop

    // result += get_time_based_value(param3);
    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = malloc(5 * sizeof(DataRecord));
    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5 {
        let mut rec = DataRecord {
            id: i,
            value: param4.wrapping_add(i.wrapping_mul(10)),
            timestamp: 0,
            name: [0 as c_char; 32],
        };
        // time(&records[i].timestamp);
        rec.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        // snprintf(records[i].name, 32, "Record_%d", i);
        let s = format!("Record_{}", i);
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(31);
        for k in 0..copy_len {
            rec.name[k] = bytes[k] as c_char;
        }
        rec.name[copy_len] = 0;
        records.push(rec);
    }

    // result += manipulate_records(records, 5, 2);
    result = result.wrapping_add(manipulate_records(&mut records, 2));

    // free(records); -- handled by Vec drop

    // result += compute_with_dynamic_memory(param1, 8);
    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    // result += global_counter + global_accumulator;
    let counter = unsafe { GLOBAL_COUNTER };
    let accumulator = unsafe { GLOBAL_ACCUMULATOR };
    result = result.wrapping_add(counter.wrapping_add(accumulator));

    result
}
