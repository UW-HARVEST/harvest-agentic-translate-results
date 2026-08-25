use std::ffi::{c_char, c_int, c_long, c_void};
use std::mem::size_of;
use std::ptr;

type OperationFunc = extern "C" fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = extern "C" fn(c_int, c_int);

static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

#[repr(C)]
pub struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: c_long,
    name: [c_char; 32],
}

const _: () = assert!(size_of::<DataRecord>() == 48);

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn time(timer: *mut c_long) -> c_long;
    fn difftime(time1: c_long, time0: c_long) -> f64;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    unsafe {
        GLOBAL_COUNTER = GLOBAL_COUNTER.wrapping_add(value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    unsafe {
        GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR.wrapping_mul(2).wrapping_add(value);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(op: OperationFunc, a: c_int, b: c_int, c: c_int) -> c_int {
    op(a, b, c)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(c)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

#[unsafe(no_mangle)]
pub extern "C" fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    let counter = unsafe { GLOBAL_COUNTER };
    a.wrapping_sub(b).wrapping_mul(c).wrapping_add(counter)
}

#[unsafe(no_mangle)]
pub extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let remaining = (size - shift_by) as usize;
        let shift = shift_by as usize;

        unsafe {
            ptr::copy(arr.add(shift), arr, remaining);
            ptr::write_bytes(arr.add(remaining), 0, shift);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    let value = unsafe { ptr.read() };
    let accumulator = unsafe { GLOBAL_ACCUMULATOR };
    value.wrapping_mul(multiplier).wrapping_add(accumulator)
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let allocation_size = (count as usize).wrapping_mul(size_of::<c_int>());
    let temp_array = unsafe { malloc(allocation_size).cast::<c_int>() };

    let mut i = 0;
    while i < count {
        let value = base.wrapping_add(i.wrapping_mul(3));
        unsafe {
            temp_array.add(i as usize).write(value);
        }
        i += 1;
    }

    let mut sum: c_int = 0;
    i = 0;
    while i < count {
        let value = unsafe { temp_array.add(i as usize).read() };
        sum = sum.wrapping_add(value);
        i += 1;
    }

    unsafe {
        free(temp_array.cast());
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let mut current_time: c_long = 0;
    unsafe {
        time(&mut current_time);
    }

    let elapsed_seconds = seed.wrapping_mul(3600);
    let reference_time = current_time.wrapping_sub(elapsed_seconds as c_long);
    let diff = unsafe { difftime(current_time, reference_time) };

    (diff / 100.0) as c_int + seed
}

#[unsafe(no_mangle)]
pub extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    if shift > 0 && shift < num_records {
        unsafe {
            ptr::copy(
                records.add(shift as usize),
                records,
                (num_records - shift) as usize,
            );
        }
    }

    let mut total: c_int = 0;
    let mut i = 0;
    while i < num_records.wrapping_sub(shift) {
        let value = unsafe { (*records.add(i as usize)).value };
        total = total.wrapping_add(value);
        i += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mod_func: ModifierFunc = increment_counter;
    mod_func(param1, 999);

    let mod_func: ModifierFunc = update_accumulator;
    mod_func(param2, 888);

    let mut result: c_int = 0;

    let op_func: OperationFunc = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    let op_func: OperationFunc = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    let op_func: OperationFunc = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));

    let dynamic_data = unsafe { malloc(10 * size_of::<c_int>()).cast::<c_int>() };
    let mut i: c_int = 0;
    while i < 10 {
        unsafe {
            dynamic_data.add(i as usize).write(param1.wrapping_add(i));
        }
        i += 1;
    }

    let selected = unsafe { dynamic_data.add(5) };
    result = result.wrapping_add(process_pointer_data(selected, param2));

    shift_array_data(dynamic_data, 10, 3);
    result = result.wrapping_add(unsafe { dynamic_data.read() });

    unsafe {
        free(dynamic_data.cast());
    }

    result = result.wrapping_add(get_time_based_value(param3));

    let records = unsafe { malloc(5 * size_of::<DataRecord>()).cast::<DataRecord>() };
    i = 0;
    while i < 5 {
        let record = unsafe { &mut *records.add(i as usize) };
        record.id = i;
        record.value = param4.wrapping_add(i.wrapping_mul(10));
        unsafe {
            time(&mut record.timestamp);
            snprintf(
                record.name.as_mut_ptr(),
                record.name.len(),
                c"Record_%d".as_ptr(),
                i,
            );
        }
        i += 1;
    }

    result = result.wrapping_add(manipulate_records(records, 5, 2));

    unsafe {
        free(records.cast());
    }

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    let counter = unsafe { GLOBAL_COUNTER };
    let accumulator = unsafe { GLOBAL_ACCUMULATOR };
    result = result.wrapping_add(counter.wrapping_add(accumulator));

    result
}
