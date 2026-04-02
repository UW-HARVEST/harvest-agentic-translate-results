use std::os::raw::c_int;

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
    a.wrapping_add(b).wrapping_add(c)
}

fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    unsafe { (a.wrapping_sub(b)).wrapping_mul(c).wrapping_add(GLOBAL_COUNTER) }
}

#[repr(C)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: libc::time_t,
    name: [u8; 32],
}

fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        unsafe {
            libc::memmove(
                arr as *mut libc::c_void,
                arr.offset(shift_by as isize) as *const libc::c_void,
                ((size - shift_by) as usize) * std::mem::size_of::<c_int>(),
            );
            libc::memset(
                arr.offset((size - shift_by) as isize) as *mut libc::c_void,
                0,
                (shift_by as usize) * std::mem::size_of::<c_int>(),
            );
        }
    }
}

fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    unsafe {
        let value = *ptr;
        value.wrapping_mul(multiplier).wrapping_add(GLOBAL_ACCUMULATOR)
    }
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    unsafe {
        let temp_array = libc::malloc((count as usize) * std::mem::size_of::<c_int>()) as *mut c_int;
        for i in 0..count {
            *temp_array.offset(i as isize) = base.wrapping_add((i * 3) as c_int);
        }
        let mut sum: c_int = 0;
        for i in 0..count {
            sum = sum.wrapping_add(*temp_array.offset(i as isize));
        }
        libc::free(temp_array as *mut libc::c_void);
        sum
    }
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

fn manipulate_records(records: *mut DataRecord, num_records: c_int, shift: c_int) -> c_int {
    unsafe {
        if shift > 0 && shift < num_records {
            libc::memmove(
                records as *mut libc::c_void,
                records.offset(shift as isize) as *const libc::c_void,
                ((num_records - shift) as usize) * std::mem::size_of::<DataRecord>(),
            );
        }
        let mut total: c_int = 0;
        for i in 0..(num_records - shift) {
            total = total.wrapping_add((*records.offset(i as isize)).value);
        }
        total
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut mod_func: ModifierFunc = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    let mut op_func: OperationFunc = add_three;
    result = result.wrapping_add(apply_operation(op_func, param1, param2, param3));

    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(op_func, param2, param3, param4));

    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(op_func, param1, param3, param4));

    unsafe {
        let dynamic_data = libc::malloc(10 * std::mem::size_of::<c_int>()) as *mut c_int;
        for i in 0..10 {
            *dynamic_data.offset(i as isize) = param1.wrapping_add(i as c_int);
        }

        result = result.wrapping_add(process_pointer_data(dynamic_data.offset(5), param2));

        shift_array_data(dynamic_data, 10, 3);
        result = result.wrapping_add(*dynamic_data.offset(0));

        libc::free(dynamic_data as *mut libc::c_void);

        result = result.wrapping_add(get_time_based_value(param3));

        let records = libc::malloc(5 * std::mem::size_of::<DataRecord>()) as *mut DataRecord;
        for i in 0..5i32 {
            let r = &mut *records.offset(i as isize);
            r.id = i;
            r.value = param4.wrapping_add((i * 10) as c_int);
            libc::time(&mut r.timestamp);
            libc::snprintf(
                r.name.as_mut_ptr() as *mut libc::c_char,
                32,
                b"Record_%d\0".as_ptr() as *const libc::c_char,
                i,
            );
        }

        result = result.wrapping_add(manipulate_records(records, 5, 2));

        libc::free(records as *mut libc::c_void);

        result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

        result = result.wrapping_add(GLOBAL_COUNTER).wrapping_add(GLOBAL_ACCUMULATOR);
    }

    result
}
