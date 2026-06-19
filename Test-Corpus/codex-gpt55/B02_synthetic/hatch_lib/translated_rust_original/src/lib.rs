use std::ffi::{c_char, c_int, c_long};
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
static GLOBAL_ACCUMULATOR: AtomicI32 = AtomicI32::new(0);

type OperationFunc = extern "C" fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = extern "C" fn(c_int, c_int);

#[repr(C)]
pub struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: c_long,
    name: [c_char; 32],
}

fn add_i32(a: c_int, b: c_int) -> c_int {
    a.wrapping_add(b)
}

fn mul_i32(a: c_int, b: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    let current = GLOBAL_COUNTER.load(Ordering::Relaxed);
    GLOBAL_COUNTER.store(add_i32(current, value), Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    let current = GLOBAL_ACCUMULATOR.load(Ordering::Relaxed);
    GLOBAL_ACCUMULATOR.store(add_i32(mul_i32(current, 2), value), Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(
    op: Option<OperationFunc>,
    a: c_int,
    b: c_int,
    c: c_int,
) -> c_int {
    op.unwrap()(a, b, c)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    add_i32(add_i32(a, b), c)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    add_i32(mul_i32(a, b), c)
}

#[unsafe(no_mangle)]
pub extern "C" fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    add_i32(mul_i32(a.wrapping_sub(b), c), GLOBAL_COUNTER.load(Ordering::Relaxed))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let count = (size - shift_by) as usize;
        let shift = shift_by as usize;
        unsafe {
            ptr::copy(arr.add(shift), arr, count);
            ptr::write_bytes(arr.add(count), 0, shift);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr_value: *mut c_int, multiplier: c_int) -> c_int {
    let value = unsafe { *ptr_value };
    add_i32(
        mul_i32(value, multiplier),
        GLOBAL_ACCUMULATOR.load(Ordering::Relaxed),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    if count <= 0 {
        return 0;
    }

    let mut temp_array: Vec<c_int> = Vec::with_capacity(count as usize);

    for i in 0..count {
        temp_array.push(add_i32(base, mul_i32(i, 3)));
    }

    let mut sum = 0;
    for value in temp_array {
        sum = add_i32(sum, value);
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let diff = seed.wrapping_mul(3600) as f64;
    add_i32((diff / 100.0) as c_int, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut total = 0;

    if shift > 0 && shift < num_records {
        unsafe {
            ptr::copy(
                records.add(shift as usize),
                records,
                (num_records - shift) as usize,
            );
        }
    }

    for i in 0..(num_records - shift) {
        let value = unsafe { (*records.add(i as usize)).value };
        total = add_i32(total, value);
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
    result = add_i32(result, apply_operation(Some(op_func), param1, param2, param3));

    op_func = multiply_add;
    result = add_i32(result, apply_operation(Some(op_func), param2, param3, param4));

    op_func = complex_calc;
    result = add_i32(result, apply_operation(Some(op_func), param1, param3, param4));

    let mut dynamic_data = Vec::with_capacity(10);
    for i in 0..10 {
        dynamic_data.push(add_i32(param1, i));
    }

    result = add_i32(
        result,
        unsafe { process_pointer_data(dynamic_data.as_mut_ptr().add(5), param2) },
    );

    unsafe {
        shift_array_data(dynamic_data.as_mut_ptr(), 10, 3);
    }
    result = add_i32(result, dynamic_data[0]);

    result = add_i32(result, get_time_based_value(param3));

    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5 {
        let mut record = DataRecord {
            id: i,
            value: add_i32(param4, mul_i32(i, 10)),
            timestamp: current_time(),
            name: [0; 32],
        };
        write_record_name(&mut record.name, i);
        records.push(record);
    }

    result = add_i32(
        result,
        unsafe { manipulate_records(records.as_mut_ptr(), 5, 2) },
    );

    result = add_i32(result, compute_with_dynamic_memory(param1, 8));

    result = add_i32(result, GLOBAL_COUNTER.load(Ordering::Relaxed));
    result = add_i32(result, GLOBAL_ACCUMULATOR.load(Ordering::Relaxed));

    result
}

fn current_time() -> c_long {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs() as c_long,
        Err(error) => -(error.duration().as_secs() as c_long),
    }
}

fn write_record_name(name: &mut [c_char; 32], index: c_int) {
    let text = match index {
        0 => b"Record_0\0".as_slice(),
        1 => b"Record_1\0".as_slice(),
        2 => b"Record_2\0".as_slice(),
        3 => b"Record_3\0".as_slice(),
        4 => b"Record_4\0".as_slice(),
        _ => return,
    };

    for (dst, src) in name.iter_mut().zip(text.iter().copied()) {
        *dst = src as c_char;
    }
}

const _: () = {
    assert!(mem::size_of::<DataRecord>() == 48);
};
