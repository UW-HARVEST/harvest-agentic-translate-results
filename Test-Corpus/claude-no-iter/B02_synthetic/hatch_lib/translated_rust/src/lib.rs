// Translation of c_src/src/lib.c to Rust.
// Produces byte-identical output to the C library's `hatch` function.

use std::ffi::c_int;
use std::os::raw::c_long;
use std::sync::Mutex;

// time_t is a signed integer of platform-specific width. On the Linux x86_64
// targets we care about it is 64 bits wide (`long`).  We model it as `c_long`
// because all that the C code does with it is plain integer arithmetic.
type TimeT = c_long;

// Mutable static state mirroring the C `static int global_counter` and
// `static int global_accumulator`.  The C code is not multi-threaded, but we
// wrap the values in a `Mutex` so that the translation only uses safe Rust
// internally.
static GLOBAL_COUNTER: Mutex<c_int> = Mutex::new(0);
static GLOBAL_ACCUMULATOR: Mutex<c_int> = Mutex::new(0);

fn increment_counter(value: c_int, _unused_param: c_int) {
    let mut g = GLOBAL_COUNTER.lock().unwrap();
    *g = g.wrapping_add(value);
}

fn update_accumulator(value: c_int, _unused_param: c_int) {
    let mut g = GLOBAL_ACCUMULATOR.lock().unwrap();
    *g = g.wrapping_mul(2).wrapping_add(value);
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

#[allow(dead_code)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: TimeT,
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
    let acc = *GLOBAL_ACCUMULATOR.lock().unwrap();
    value.wrapping_mul(multiplier).wrapping_add(acc)
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let count_usize = count as usize;
    let mut temp_array: Vec<c_int> = vec![0; count_usize];

    for i in 0..count {
        temp_array[i as usize] = base.wrapping_add(i.wrapping_mul(3));
    }

    let mut sum: c_int = 0;
    for i in 0..count {
        sum = sum.wrapping_add(temp_array[i as usize]);
    }

    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    // The C code does:
    //     time(&current_time);
    //     reference_time = current_time - (seed * 3600);
    //     diff = difftime(current_time, reference_time);
    //     return (int)(diff / 100) + seed;
    //
    // `seed * 3600` is an int multiplication that wraps (in practice) on
    // overflow; the result is then sign-extended to `time_t` for the
    // subtraction.  difftime then yields exactly that same value as a
    // double, regardless of what `time()` returned.  So the result is
    // deterministic and matches:
    //     (int)((double)(time_t)(int)(seed * 3600) / 100) + seed
    let product_int: c_int = seed.wrapping_mul(3600);
    let product_time: TimeT = product_int as TimeT;
    let diff: f64 = product_time as f64;
    ((diff / 100.0) as c_int).wrapping_add(seed)
}

fn manipulate_records(records: &mut [DataRecord], num_records: c_int, shift: c_int) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        let shift_usize = shift as usize;
        let num_usize = num_records as usize;
        // memmove(records, records + shift, ...)
        // We need to move records[shift..num_records] to records[0..num_records-shift].
        // We can't trivially copy_within for non-Copy types, so emulate the
        // C memmove by std::mem::swap into temporaries — but simpler is to
        // shift the fields manually.  Records have plain numeric fields and
        // a fixed-size byte array, all of which are Copy when bundled.
        for i in 0..(num_usize - shift_usize) {
            let src = i + shift_usize;
            // Move the fields one record at a time.
            let id = records[src].id;
            let value = records[src].value;
            let timestamp = records[src].timestamp;
            let name = records[src].name;
            records[i].id = id;
            records[i].value = value;
            records[i].timestamp = timestamp;
            records[i].name = name;
        }
    }

    for i in 0..(num_records - shift) {
        total = total.wrapping_add(records[i as usize].value);
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

    // int *dynamic_data = (int *)malloc(10 * sizeof(int));
    let mut dynamic_data: Vec<c_int> = vec![0; 10];
    for i in 0..10i32 {
        dynamic_data[i as usize] = param1.wrapping_add(i);
    }

    // result += process_pointer_data(&dynamic_data[5], param2);
    result = result.wrapping_add(process_pointer_data(dynamic_data[5], param2));

    // shift_array_data(dynamic_data, 10, 3);
    shift_array_data(&mut dynamic_data, 10, 3);
    result = result.wrapping_add(dynamic_data[0]);

    drop(dynamic_data);

    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = (DataRecord *)malloc(5 * sizeof(DataRecord));
    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5i32 {
        let mut name = [0u8; 32];
        // snprintf(records[i].name, 32, "Record_%d", i);
        let s = format!("Record_{}", i);
        let bytes = s.as_bytes();
        let n = bytes.len().min(31);
        name[..n].copy_from_slice(&bytes[..n]);
        // name[n] is already 0 from the initialiser.
        records.push(DataRecord {
            id: i,
            value: param4.wrapping_add(i.wrapping_mul(10)),
            timestamp: 0, // The actual time is unused after assignment.
            name,
        });
    }

    result = result.wrapping_add(manipulate_records(&mut records, 5, 2));

    drop(records);

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    let counter = *GLOBAL_COUNTER.lock().unwrap();
    let accumulator = *GLOBAL_ACCUMULATOR.lock().unwrap();
    result = result.wrapping_add(counter.wrapping_add(accumulator));

    result
}
