use std::ffi::c_int;

static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

type OperationFunc = fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = fn(c_int, c_int);

fn increment_counter(value: c_int, _unused: c_int) {
    unsafe { GLOBAL_COUNTER += value; }
}

fn update_accumulator(value: c_int, _unused: c_int) {
    unsafe { GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR * 2 + value; }
}

fn apply_operation(op: OperationFunc, a: c_int, b: c_int, c: c_int) -> c_int {
    op(a, b, c)
}

fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a + b + c
}

fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    unsafe { (a - b).wrapping_mul(c).wrapping_add(GLOBAL_COUNTER) }
}

#[repr(C)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: libc::time_t,
    name: [u8; 32],
}

fn shift_array_data(arr: &mut [c_int], shift_by: usize) {
    let size = arr.len();
    if shift_by > 0 && shift_by < size {
        arr.copy_within(shift_by.., 0);
        for i in (size - shift_by)..size {
            arr[i] = 0;
        }
    }
}

fn process_pointer_data(value: c_int, multiplier: c_int) -> c_int {
    unsafe { value.wrapping_mul(multiplier).wrapping_add(GLOBAL_ACCUMULATOR) }
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    for i in 0..count {
        sum += base + i * 3;
    }
    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    unsafe {
        let mut current_time: libc::time_t = 0;
        libc::time(&mut current_time);
        let reference_time = current_time - (seed as libc::time_t) * 3600;
        let diff = libc::difftime(current_time, reference_time);
        (diff / 100.0) as c_int + seed
    }
}

fn manipulate_records(records: &mut [DataRecord], shift: usize) -> c_int {
    let num = records.len();
    if shift > 0 && shift < num {
        unsafe {
            let ptr = records.as_mut_ptr();
            std::ptr::copy(ptr.add(shift), ptr, num - shift);
        }
    }
    let mut total: c_int = 0;
    for i in 0..(num - shift) {
        total += records[i].value;
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mod_func: ModifierFunc = increment_counter;
    mod_func(param1, 999);

    let mod_func: ModifierFunc = update_accumulator;
    mod_func(param2, 888);

    let op_func: OperationFunc = add_three;
    result += apply_operation(op_func, param1, param2, param3);

    let op_func: OperationFunc = multiply_add;
    result += apply_operation(op_func, param2, param3, param4);

    let op_func: OperationFunc = complex_calc;
    result += apply_operation(op_func, param1, param3, param4);

    let mut dynamic_data = vec![0i32; 10];
    for i in 0..10 {
        dynamic_data[i] = param1 + i as c_int;
    }

    result += process_pointer_data(dynamic_data[5], param2);

    shift_array_data(&mut dynamic_data, 3);
    result += dynamic_data[0];

    result += get_time_based_value(param3);

    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5 {
        let mut rec = DataRecord {
            id: i,
            value: param4 + i * 10,
            timestamp: 0,
            name: [0u8; 32],
        };
        unsafe { libc::time(&mut rec.timestamp); }
        let s = format!("Record_{}", i);
        let bytes = s.as_bytes();
        let len = bytes.len().min(31);
        rec.name[..len].copy_from_slice(&bytes[..len]);
        records.push(rec);
    }

    result += manipulate_records(&mut records, 2);

    result += compute_with_dynamic_memory(param1, 8);

    unsafe { result += GLOBAL_COUNTER + GLOBAL_ACCUMULATOR; }

    result
}
