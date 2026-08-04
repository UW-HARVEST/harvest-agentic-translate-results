use std::io::Write;
use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
static GLOBAL_ACCUMULATOR: AtomicI32 = AtomicI32::new(0);

type OperationFunc = fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = fn(c_int, c_int);

fn increment_counter(value: c_int, _unused_param: c_int) {
    GLOBAL_COUNTER.fetch_add(value as i32, Ordering::SeqCst);
}

fn update_accumulator(value: c_int, _unused_param: c_int) {
    let _ = GLOBAL_ACCUMULATOR.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |acc| {
        Some(acc * 2 + value as i32)
    });
}

fn apply_operation(op: OperationFunc, a: c_int, b: c_int, c: c_int) -> c_int {
    op(a, b, c)
}

fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a + b + c
}

fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    (a * b) + c
}

fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    (a - b) * c + GLOBAL_COUNTER.load(Ordering::SeqCst) as c_int
}

#[repr(C)]
#[derive(Clone)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64,
    name: [u8; 32],
}

fn shift_array_data(arr: &mut [c_int], shift_by: usize) {
    let size = arr.len();
    if shift_by > 0 && shift_by < size {
        arr.copy_within(shift_by..size, 0);
        arr[(size - shift_by)..].fill(0);
    }
}

fn process_pointer_data(ptr: &c_int, multiplier: c_int) -> c_int {
    let value = *ptr;
    value * multiplier + GLOBAL_ACCUMULATOR.load(Ordering::SeqCst) as c_int
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    if count <= 0 {
        return 0;
    }
    let count = count as usize;
    let mut temp_array = vec![0; count];

    for i in 0..count {
        temp_array[i] = base + (i as c_int) * 3;
    }

    let mut sum = 0;
    for i in 0..count {
        sum += temp_array[i];
    }

    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let reference_time = current_time - (seed as i64 * 3600);
    let diff = (current_time - reference_time) as f64;
    (diff / 100.0) as c_int + seed
}

fn manipulate_records(records: &mut [DataRecord], shift: usize) -> c_int {
    let num_records = records.len();
    let mut total = 0;

    if shift > 0 && shift < num_records {
        records.copy_within(shift..num_records, 0);
    }

    let limit = if shift < num_records {
        num_records - shift
    } else {
        0
    };

    for i in 0..limit {
        total += records[i].value;
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result = 0;

    let mut mod_func: ModifierFunc;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    let mut op_func: OperationFunc;

    op_func = add_three;
    result += apply_operation(op_func, param1, param2, param3);

    op_func = multiply_add;
    result += apply_operation(op_func, param2, param3, param4);

    op_func = complex_calc;
    result += apply_operation(op_func, param1, param3, param4);

    let mut dynamic_data = vec![0; 10];
    for i in 0..10 {
        dynamic_data[i] = param1 + i as c_int;
    }

    result += process_pointer_data(&dynamic_data[5], param2);

    shift_array_data(&mut dynamic_data, 3);
    result += dynamic_data[0];

    result += get_time_based_value(param3);

    let mut records = vec![
        DataRecord {
            id: 0,
            value: 0,
            timestamp: 0,
            name: [0; 32],
        };
        5
    ];

    for i in 0..5 {
        records[i].id = i as c_int;
        records[i].value = param4 + (i as c_int) * 10;
        records[i].timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let mut cursor = std::io::Cursor::new(&mut records[i].name[..]);
        let _ = write!(cursor, "Record_{}\0", i);
    }

    result += manipulate_records(&mut records, 2);

    result += compute_with_dynamic_memory(param1, 8);

    result += GLOBAL_COUNTER.load(Ordering::SeqCst) as c_int
        + GLOBAL_ACCUMULATOR.load(Ordering::SeqCst) as c_int;

    result
}
