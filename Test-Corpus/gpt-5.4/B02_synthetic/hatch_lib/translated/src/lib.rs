use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
static GLOBAL_ACCUMULATOR: AtomicI32 = AtomicI32::new(0);

type OperationFunc = fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = fn(c_int, c_int);

fn increment_counter(value: c_int, _unused_param: c_int) {
    GLOBAL_COUNTER.fetch_add(value, Ordering::Relaxed);
}

fn update_accumulator(value: c_int, _unused_param: c_int) {
    let _ = GLOBAL_ACCUMULATOR.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.wrapping_mul(2).wrapping_add(value))
    });
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
    a.wrapping_sub(b)
        .wrapping_mul(c)
        .wrapping_add(GLOBAL_COUNTER.load(Ordering::Relaxed))
}

#[derive(Clone)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64,
    name: [u8; 32],
}

fn shift_array_data(arr: &mut [c_int], shift_by: c_int) {
    let size = arr.len() as c_int;
    if shift_by > 0 && shift_by < size {
        let shift = shift_by as usize;
        arr.copy_within(shift.., 0);
        for item in &mut arr[(size as usize - shift)..] {
            *item = 0;
        }
    }
}

fn process_pointer_data(ptr: &c_int, multiplier: c_int) -> c_int {
    ptr.wrapping_mul(multiplier)
        .wrapping_add(GLOBAL_ACCUMULATOR.load(Ordering::Relaxed))
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let count_usize = if count <= 0 { 0 } else { count as usize };
    let temp_array: Vec<c_int> = (0..count_usize)
        .map(|i| base.wrapping_add((i as c_int).wrapping_mul(3)))
        .collect();

    temp_array
        .into_iter()
        .fold(0, |acc, v| acc.wrapping_add(v))
}

fn get_time_based_value(seed: c_int) -> c_int {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let reference_time = current_time.wrapping_sub((seed as i64).wrapping_mul(3600));
    let diff = current_time.wrapping_sub(reference_time);
    ((diff / 100) as c_int).wrapping_add(seed)
}

fn manipulate_records(records: &mut [DataRecord], shift: c_int) -> c_int {
    let num_records = records.len() as c_int;
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        records.copy_within(shift as usize.., 0);
    }

    let limit = num_records.wrapping_sub(shift);
    if limit > 0 {
        for record in records.iter().take(limit as usize) {
            total = total.wrapping_add(record.value);
        }
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

    let mut dynamic_data: Vec<c_int> = (0..10)
        .map(|i| param1.wrapping_add(i))
        .collect();

    result = result.wrapping_add(process_pointer_data(&dynamic_data[5], param2));

    shift_array_data(&mut dynamic_data, 3);
    result = result.wrapping_add(dynamic_data[0]);

    result = result.wrapping_add(get_time_based_value(param3));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut records: Vec<DataRecord> = (0..5)
        .map(|i| {
            let mut name = [0u8; 32];
            let s = format!("Record_{}", i);
            let bytes = s.as_bytes();
            let len = bytes.len().min(31);
            name[..len].copy_from_slice(&bytes[..len]);
            DataRecord {
                id: i,
                value: param4.wrapping_add(i.wrapping_mul(10)),
                timestamp: now,
                name,
            }
        })
        .collect();

    result = result.wrapping_add(manipulate_records(&mut records, 2));

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    result = result.wrapping_add(GLOBAL_COUNTER.load(Ordering::Relaxed));
    result = result.wrapping_add(GLOBAL_ACCUMULATOR.load(Ordering::Relaxed));

    result
}
