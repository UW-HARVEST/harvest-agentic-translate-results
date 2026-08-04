use std::ffi::c_int;

static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

#[repr(C)]
pub struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: libc::time_t,
    name: [u8; 32],
}

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused: c_int) {
    unsafe { GLOBAL_COUNTER += value; }
}

#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused: c_int) {
    unsafe { GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR * 2 + value; }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(
    op: Option<extern "C" fn(c_int, c_int, c_int) -> c_int>,
    a: c_int, b: c_int, c: c_int,
) -> c_int {
    (op.unwrap())(a, b, c)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a + b + c
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

#[unsafe(no_mangle)]
pub extern "C" fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    unsafe { (a - b).wrapping_mul(c).wrapping_add(GLOBAL_COUNTER) }
}

#[unsafe(no_mangle)]
pub extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        unsafe {
            let shift = shift_by as usize;
            let sz = size as usize;
            std::ptr::copy(arr.add(shift), arr, sz - shift);
            std::ptr::write_bytes(arr.add(sz - shift), 0, shift);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_pointer_data(ptr: *const c_int, multiplier: c_int) -> c_int {
    unsafe {
        let value = *ptr;
        value.wrapping_mul(multiplier).wrapping_add(GLOBAL_ACCUMULATOR)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    for i in 0..count {
        sum += base + i * 3;
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    unsafe {
        let mut current_time: libc::time_t = 0;
        libc::time(&mut current_time);
        let reference_time = current_time - (seed as libc::time_t) * 3600;
        let diff = libc::difftime(current_time, reference_time);
        (diff / 100.0) as c_int + seed
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn manipulate_records(records: *mut DataRecord, num_records: c_int, shift: c_int) -> c_int {
    let num = num_records as usize;
    let sh = shift as usize;
    if shift > 0 && sh < num {
        unsafe {
            std::ptr::copy(records.add(sh), records, num - sh);
        }
    }
    let mut total: c_int = 0;
    for i in 0..(num - sh) {
        unsafe { total += (*records.add(i)).value; }
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    increment_counter(param1, 999);
    update_accumulator(param2, 888);

    result += apply_operation(Some(add_three), param1, param2, param3);
    result += apply_operation(Some(multiply_add), param2, param3, param4);
    result += apply_operation(Some(complex_calc), param1, param3, param4);

    let mut dynamic_data = vec![0i32; 10];
    for i in 0..10 {
        dynamic_data[i] = param1 + i as c_int;
    }

    result += process_pointer_data(&dynamic_data[5], param2);

    shift_array_data(dynamic_data.as_mut_ptr(), 10, 3);
    result += dynamic_data[0];

    result += get_time_based_value(param3);

    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5i32 {
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

    result += manipulate_records(records.as_mut_ptr(), 5, 2);

    result += compute_with_dynamic_memory(param1, 8);

    unsafe { result += GLOBAL_COUNTER + GLOBAL_ACCUMULATOR; }

    result
}
