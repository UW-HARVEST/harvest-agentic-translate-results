// Rust translation of c_src/src/lib.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_void};

/// `time_t` on LP64 platforms (x86-64 / aarch64 Linux): a signed 64-bit integer.
type time_t = i64;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn time(t: *mut time_t) -> time_t;
    fn difftime(time1: time_t, time0: time_t) -> c_double;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
}

// static int global_counter = 0;
// static int global_accumulator = 0;
static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

// typedef int (*operation_func)(int, int, int);
type operation_func = extern "C" fn(c_int, c_int, c_int) -> c_int;
// typedef void (*modifier_func)(int, int);
type modifier_func = extern "C" fn(c_int, c_int);

// void increment_counter(int value, int unused_param)
#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    unsafe {
        GLOBAL_COUNTER = GLOBAL_COUNTER.wrapping_add(value);
    }
}

// void update_accumulator(int value, int unused_param)
#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    unsafe {
        GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR.wrapping_mul(2).wrapping_add(value);
    }
}

// int apply_operation(operation_func op, int a, int b, int c)
//
// The function pointer is taken as an opaque pointer so that the exact C ABI
// (and the exact behaviour of a call through an invalid pointer) is preserved.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(
    op: *const c_void,
    a: c_int,
    b: c_int,
    c: c_int,
) -> c_int {
    let op: operation_func = std::mem::transmute::<*const c_void, operation_func>(op);
    op(a, b, c)
}

// int add_three(int a, int b, int c)
#[unsafe(no_mangle)]
pub extern "C" fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(c)
}

// int multiply_add(int a, int b, int c)
#[unsafe(no_mangle)]
pub extern "C" fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

// int complex_calc(int a, int b, int c)
#[unsafe(no_mangle)]
pub extern "C" fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    unsafe { a.wrapping_sub(b).wrapping_mul(c).wrapping_add(GLOBAL_COUNTER) }
}

// typedef struct {
//     int id;
//     int value;
//     time_t timestamp;
//     char name[32];
// } DataRecord;
//
// Layout on LP64: size 48, align 8, offsets 0 / 4 / 8 / 16.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: time_t,
    pub name: [c_char; 32],
}

// void shift_array_data(int *arr, int size, int shift_by)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        memmove(
            arr as *mut c_void,
            arr.offset(shift_by as isize) as *const c_void,
            c_int_to_size(size.wrapping_sub(shift_by))
                .wrapping_mul(std::mem::size_of::<c_int>()),
        );
        memset(
            arr.offset(size.wrapping_sub(shift_by) as isize) as *mut c_void,
            0,
            c_int_to_size(shift_by).wrapping_mul(std::mem::size_of::<c_int>()),
        );
    }
}

// int process_pointer_data(int *ptr, int multiplier)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    let value: c_int = *ptr;
    value.wrapping_mul(multiplier).wrapping_add(GLOBAL_ACCUMULATOR)
}

// int compute_with_dynamic_memory(int base, int count)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let temp_array =
        malloc(c_int_to_size(count).wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;

    let mut i: c_int = 0;
    while i < count {
        *temp_array.offset(i as isize) = base.wrapping_add(i.wrapping_mul(3));
        i = i.wrapping_add(1);
    }

    let mut sum: c_int = 0;
    let mut i: c_int = 0;
    while i < count {
        sum = sum.wrapping_add(*temp_array.offset(i as isize));
        i = i.wrapping_add(1);
    }

    free(temp_array as *mut c_void);

    sum
}

// int get_time_based_value(int seed)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let mut current_time: time_t = 0;
    let reference_time: time_t;

    time(&mut current_time);

    // `seed * 3600` is evaluated in `int` and only then converted to time_t.
    reference_time = current_time.wrapping_sub(seed.wrapping_mul(3600) as time_t);

    let diff: c_double = difftime(current_time, reference_time);

    ((diff / 100.0) as c_int).wrapping_add(seed)
}

// int manipulate_records(DataRecord *records, int num_records, int shift)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        memmove(
            records as *mut c_void,
            records.offset(shift as isize) as *const c_void,
            c_int_to_size(num_records.wrapping_sub(shift))
                .wrapping_mul(std::mem::size_of::<DataRecord>()),
        );
    }

    let mut i: c_int = 0;
    while i < num_records.wrapping_sub(shift) {
        total = total.wrapping_add((*records.offset(i as isize)).value);
        i = i.wrapping_add(1);
    }

    total
}

// int hatch(int param1, int param2, int param3, int param4)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hatch(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let mut mod_func: modifier_func;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    let mut op_func: operation_func;

    op_func = add_three;
    result = result.wrapping_add(apply_operation(
        op_func as *const c_void,
        param1,
        param2,
        param3,
    ));

    op_func = multiply_add;
    result = result.wrapping_add(apply_operation(
        op_func as *const c_void,
        param2,
        param3,
        param4,
    ));

    op_func = complex_calc;
    result = result.wrapping_add(apply_operation(
        op_func as *const c_void,
        param1,
        param3,
        param4,
    ));

    let dynamic_data = malloc(10 * std::mem::size_of::<c_int>()) as *mut c_int;
    let mut i: c_int = 0;
    while i < 10 {
        *dynamic_data.offset(i as isize) = param1.wrapping_add(i);
        i = i.wrapping_add(1);
    }

    result = result.wrapping_add(process_pointer_data(dynamic_data.offset(5), param2));

    shift_array_data(dynamic_data, 10, 3);
    result = result.wrapping_add(*dynamic_data.offset(0));

    free(dynamic_data as *mut c_void);

    result = result.wrapping_add(get_time_based_value(param3));

    let records = malloc(5 * std::mem::size_of::<DataRecord>()) as *mut DataRecord;

    let mut i: c_int = 0;
    while i < 5 {
        let rec = records.offset(i as isize);
        (*rec).id = i;
        (*rec).value = param4.wrapping_add(i.wrapping_mul(10));
        time(&mut (*rec).timestamp);
        snprintf(
            (*rec).name.as_mut_ptr(),
            32,
            b"Record_%d\0".as_ptr() as *const c_char,
            i,
        );
        i = i.wrapping_add(1);
    }

    result = result.wrapping_add(manipulate_records(records, 5, 2));

    free(records as *mut c_void);

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    result = result
        .wrapping_add(GLOBAL_COUNTER.wrapping_add(GLOBAL_ACCUMULATOR));

    result
}

/// Reproduces the C conversion of an `int` to `size_t` (modulo 2^64, i.e. sign
/// extension on LP64) that happens implicitly in `count * sizeof(...)`.
#[inline]
fn c_int_to_size(v: c_int) -> usize {
    v as isize as usize
}
