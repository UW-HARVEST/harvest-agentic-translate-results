// Translation of c_src/src/lib.c — preserves exact byte-for-byte behavior.

use std::ffi::c_int;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicI32, Ordering};

// Globals (file-level `static int` in C). We use AtomicI32 with Relaxed ordering
// to provide non-atomic-like single-thread semantics that match the original.
static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
static GLOBAL_ACCUMULATOR: AtomicI32 = AtomicI32::new(0);

// In C: typedef int (*operation_func)(int, int, int);
type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
// In C: typedef void (*modifier_func)(int, int);
type ModifierFunc = unsafe extern "C" fn(c_int, c_int);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: libc_time_t,
    name: [c_char; 32],
}

// time_t is typically a 64-bit signed integer on Linux x86_64.
#[allow(non_camel_case_types)]
type libc_time_t = i64;

extern "C" {
    fn time(tloc: *mut libc_time_t) -> libc_time_t;
    fn difftime(time1: libc_time_t, time0: libc_time_t) -> f64;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
    fn memmove(
        dest: *mut std::ffi::c_void,
        src: *const std::ffi::c_void,
        n: usize,
    ) -> *mut std::ffi::c_void;
    fn memset(s: *mut std::ffi::c_void, c: c_int, n: usize) -> *mut std::ffi::c_void;
}

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    let cur = GLOBAL_COUNTER.load(Ordering::Relaxed);
    GLOBAL_COUNTER.store(cur.wrapping_add(value), Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    let cur = GLOBAL_ACCUMULATOR.load(Ordering::Relaxed);
    GLOBAL_ACCUMULATOR.store(cur.wrapping_mul(2).wrapping_add(value), Ordering::Relaxed);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    op: OperationFunc,
    a: c_int,
    b: c_int,
    c: c_int,
) -> c_int {
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
    a.wrapping_sub(b)
        .wrapping_mul(c)
        .wrapping_add(GLOBAL_COUNTER.load(Ordering::Relaxed))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let elem_size = std::mem::size_of::<c_int>();
        memmove(
            arr as *mut std::ffi::c_void,
            arr.offset(shift_by as isize) as *const std::ffi::c_void,
            (size - shift_by) as usize * elem_size,
        );
        memset(
            arr.offset((size - shift_by) as isize) as *mut std::ffi::c_void,
            0,
            shift_by as usize * elem_size,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    let value = *ptr;
    value
        .wrapping_mul(multiplier)
        .wrapping_add(GLOBAL_ACCUMULATOR.load(Ordering::Relaxed))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let elem_size = std::mem::size_of::<c_int>();
    let temp_array = malloc(count as usize * elem_size) as *mut c_int;

    for i in 0..count {
        *temp_array.offset(i as isize) = base.wrapping_add(i.wrapping_mul(3));
    }

    let mut sum: c_int = 0;
    for i in 0..count {
        sum = sum.wrapping_add(*temp_array.offset(i as isize));
    }

    free(temp_array as *mut std::ffi::c_void);

    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let mut current_time: libc_time_t = 0;
    let reference_time: libc_time_t;

    time(&mut current_time as *mut libc_time_t);

    reference_time = current_time - (seed as libc_time_t * 3600);

    let diff: f64 = difftime(current_time, reference_time);

    (diff / 100.0) as c_int + seed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        let elem_size = std::mem::size_of::<DataRecord>();
        memmove(
            records as *mut std::ffi::c_void,
            records.offset(shift as isize) as *const std::ffi::c_void,
            (num_records - shift) as usize * elem_size,
        );
    }

    for i in 0..(num_records - shift) {
        total = total.wrapping_add((*records.offset(i as isize)).value);
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

    let elem_size = std::mem::size_of::<c_int>();
    let dynamic_data = malloc(10 * elem_size) as *mut c_int;
    for i in 0..10 {
        *dynamic_data.offset(i as isize) = param1.wrapping_add(i as c_int);
    }

    result = result.wrapping_add(process_pointer_data(dynamic_data.offset(5), param2));

    shift_array_data(dynamic_data, 10, 3);
    result = result.wrapping_add(*dynamic_data.offset(0));

    free(dynamic_data as *mut std::ffi::c_void);

    result = result.wrapping_add(get_time_based_value(param3));

    let rec_size = std::mem::size_of::<DataRecord>();
    let records = malloc(5 * rec_size) as *mut DataRecord;

    for i in 0..5i32 {
        (*records.offset(i as isize)).id = i;
        (*records.offset(i as isize)).value = param4.wrapping_add(i.wrapping_mul(10));
        time(&mut (*records.offset(i as isize)).timestamp as *mut libc_time_t);
        let fmt = b"Record_%d\0";
        snprintf(
            (*records.offset(i as isize)).name.as_mut_ptr(),
            32,
            fmt.as_ptr() as *const c_char,
            i,
        );
    }

    result = result.wrapping_add(manipulate_records(records, 5, 2));

    free(records as *mut std::ffi::c_void);

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    result = result
        .wrapping_add(GLOBAL_COUNTER.load(Ordering::Relaxed))
        .wrapping_add(GLOBAL_ACCUMULATOR.load(Ordering::Relaxed));

    result
}
