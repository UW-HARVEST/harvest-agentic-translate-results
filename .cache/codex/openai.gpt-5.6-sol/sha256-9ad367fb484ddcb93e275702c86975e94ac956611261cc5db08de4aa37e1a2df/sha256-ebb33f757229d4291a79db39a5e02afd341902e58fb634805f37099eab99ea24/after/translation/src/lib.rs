use std::ffi::{c_char, c_double, c_int, c_long, c_void};
use std::mem::size_of;
use std::ptr;

type TimeT = c_long;
type OperationFunc = extern "C" fn(c_int, c_int, c_int) -> c_int;
type ModifierFunc = extern "C" fn(c_int, c_int);

static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

#[repr(C)]
pub struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: TimeT,
    name: [c_char; 32],
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
    fn memset(destination: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn time(timer: *mut TimeT) -> TimeT;
    fn difftime(time1: TimeT, time0: TimeT) -> c_double;
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
pub extern "C" fn apply_operation(operation: OperationFunc, a: c_int, b: c_int, c: c_int) -> c_int {
    operation(a, b, c)
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
    let calculated = a.wrapping_sub(b).wrapping_mul(c);
    unsafe { calculated.wrapping_add(GLOBAL_COUNTER) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let retained = size - shift_by;
        unsafe {
            memmove(
                arr.cast(),
                arr.add(shift_by as usize).cast(),
                (retained as usize).wrapping_mul(size_of::<c_int>()),
            );
            memset(
                arr.add(retained as usize).cast(),
                0,
                (shift_by as usize).wrapping_mul(size_of::<c_int>()),
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    let value = unsafe { ptr.read() };
    let accumulator = unsafe { GLOBAL_ACCUMULATOR };
    value.wrapping_mul(multiplier).wrapping_add(accumulator)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let allocation_size = (count as usize).wrapping_mul(size_of::<c_int>());
    let temp_array = unsafe { malloc(allocation_size) }.cast::<c_int>();

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
    let mut current_time: TimeT = 0;
    unsafe {
        time(&mut current_time);
    }

    let reference_time = current_time.wrapping_sub(seed.wrapping_mul(3600) as TimeT);
    let difference = unsafe { difftime(current_time, reference_time) };

    ((difference / 100.0) as c_int).wrapping_add(seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let remaining = num_records.wrapping_sub(shift);

    if shift > 0 && shift < num_records {
        unsafe {
            memmove(
                records.cast(),
                records.add(shift as usize).cast(),
                (remaining as usize).wrapping_mul(size_of::<DataRecord>()),
            );
        }
    }

    let mut total: c_int = 0;
    let mut i = 0;
    while i < remaining {
        let value = unsafe { (*records.add(i as usize)).value };
        total = total.wrapping_add(value);
        i += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hatch(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let mut modifier: ModifierFunc = increment_counter;
    modifier(param1, 999);

    modifier = update_accumulator;
    modifier(param2, 888);

    let mut operation: OperationFunc = add_three;
    result = result.wrapping_add(apply_operation(operation, param1, param2, param3));

    operation = multiply_add;
    result = result.wrapping_add(apply_operation(operation, param2, param3, param4));

    operation = complex_calc;
    result = result.wrapping_add(apply_operation(operation, param1, param3, param4));

    let dynamic_data = unsafe { malloc(10 * size_of::<c_int>()) }.cast::<c_int>();
    for i in 0..10 {
        unsafe {
            dynamic_data.add(i).write(param1.wrapping_add(i as c_int));
        }
    }

    result = result.wrapping_add(unsafe { process_pointer_data(dynamic_data.add(5), param2) });

    unsafe {
        shift_array_data(dynamic_data, 10, 3);
        result = result.wrapping_add(dynamic_data.read());
        free(dynamic_data.cast());
    }

    result = result.wrapping_add(get_time_based_value(param3));

    let records = unsafe { malloc(5 * size_of::<DataRecord>()) }.cast::<DataRecord>();
    for i in 0..5 {
        let record = unsafe { records.add(i) };
        unsafe {
            ptr::addr_of_mut!((*record).id).write(i as c_int);
            ptr::addr_of_mut!((*record).value)
                .write(param4.wrapping_add((i as c_int).wrapping_mul(10)));
            time(ptr::addr_of_mut!((*record).timestamp));
            snprintf(
                ptr::addr_of_mut!((*record).name).cast(),
                32,
                c"Record_%d".as_ptr(),
                i as c_int,
            );
        }
    }

    result = result.wrapping_add(unsafe { manipulate_records(records, 5, 2) });

    unsafe {
        free(records.cast());
    }

    result = result.wrapping_add(unsafe { compute_with_dynamic_memory(param1, 8) });

    let counter = unsafe { GLOBAL_COUNTER };
    let accumulator = unsafe { GLOBAL_ACCUMULATOR };
    result.wrapping_add(counter).wrapping_add(accumulator)
}
