









use std::convert::TryFrom;

use std::time::{SystemTime, UNIX_EPOCH};

extern "C" {
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn time(__timer: *mut time_t) -> time_t;
    fn difftime(__time1: time_t, __time0: time_t) -> ::core::ffi::c_double;
}
pub type size_t = usize;
pub type __time_t = ::core::ffi::c_long;
pub type time_t = __time_t;
pub type operation_func = Option<
    unsafe extern "C" fn(
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        ::core::ffi::c_int,
    ) -> ::core::ffi::c_int,
>;
pub type modifier_func = Option<unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataRecord {
    pub id: ::core::ffi::c_int,
    pub value: ::core::ffi::c_int,
    pub timestamp: time_t,
    pub name: [::core::ffi::c_char; 32],
}
static mut global_counter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut global_accumulator: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn increment_counter(value: ::core::ffi::c_int, _unused_param: ::core::ffi::c_int) {
    global_counter += value;
}

#[no_mangle]
pub unsafe extern "C" fn update_accumulator(value: i32, _unused_param: i32) {
    global_accumulator = global_accumulator * 2 + value;
}

#[no_mangle]
pub fn apply_operation(
    op: Option<unsafe extern "C" fn(i32, i32, i32) -> i32>,
    a: i32,
    b: i32,
    c: i32,
) -> i32 {
    let op = op.expect("non-null function pointer");
    unsafe { op(a, b, c) }
}

#[no_mangle]
pub unsafe extern "C" fn add_three(
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
    c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    a + b + c
}

#[no_mangle]
pub unsafe extern "C" fn multiply_add(a: ::core::ffi::c_int, b: ::core::ffi::c_int, c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    a * b + c
}

#[no_mangle]
pub unsafe extern "C" fn complex_calc(a: i32, b: i32, c: i32) -> i32 {
    (a - b) * c + global_counter
}

#[no_mangle]
pub fn shift_array_data(arr: &mut [i32], shift_by: i32) {
    if shift_by > 0 {
        let shift_by = shift_by as usize;
        let size = arr.len();
        if shift_by < size {
            arr.copy_within(shift_by..size, 0);
            arr[size - shift_by..].fill(0);
        }
    }
}

#[no_mangle]
pub fn process_pointer_data(ptr: &i32, multiplier: i32) -> i32 {
    let value = *ptr;
    value * multiplier + unsafe { global_accumulator }
}

#[no_mangle]
pub fn compute_with_dynamic_memory(base: i32, count: i32) -> i32 {
    let mut temp_array = Vec::with_capacity(count.max(0) as usize);
    for i in 0..count {
        temp_array.push(base + i * 3);
    }
    temp_array.iter().sum()
}

#[no_mangle]
pub fn get_time_based_value(seed: i32) -> i32 {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let reference_time = current_time - (seed as i64 * 3600);
    let diff = (current_time - reference_time) as f64;

    (diff / 100.0) as i32 + seed
}

#[no_mangle]
pub fn manipulate_records(records: &mut [DataRecord], shift: i32) -> i32 {
    let num_records = records.len();
    let shift = usize::try_from(shift).ok().unwrap_or(0);

    if shift > 0 && shift < num_records {
        records.copy_within(shift..num_records, 0);
    }

    records[..num_records.saturating_sub(shift)]
        .iter()
        .map(|record| record.value)
        .sum()
}

#[no_mangle]
pub fn hatch(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result = 0;

    let mut mod_func: modifier_func = Some(increment_counter);
    if let Some(f) = mod_func {
        unsafe {
            f(param1, 999);
        }
    }

    mod_func = Some(update_accumulator);
    if let Some(f) = mod_func {
        unsafe {
            f(param2, 888);
        }
    }

    let mut op_func: operation_func = Some(add_three);
    result += apply_operation(op_func, param1, param2, param3);

    op_func = Some(multiply_add);
    result += apply_operation(op_func, param2, param3, param4);

    op_func = Some(complex_calc);
    result += apply_operation(op_func, param1, param3, param4);

    let mut dynamic_data: Vec<i32> = (0..10).map(|i| param1 + i).collect();

    result += process_pointer_data(&mut dynamic_data[5], param2);

    shift_array_data(&mut dynamic_data, 3);
    result += dynamic_data[0];

    result += get_time_based_value(param3);

    let mut records: Vec<DataRecord> = (0..5)
        .map(|i| {
            let mut name = [0i8; 32];
            let record_name = format!("Record_{i}");
            for (dst, src) in name.iter_mut().zip(record_name.bytes()) {
                *dst = src as i8;
            }

            DataRecord {
                id: i,
                value: param4 + i * 10,
                timestamp: 0,
                name,
            }
        })
        .collect();

    result += manipulate_records(&mut records, 2);

    result += compute_with_dynamic_memory(param1, 8);

    result += unsafe { global_counter + global_accumulator };

    result
}

