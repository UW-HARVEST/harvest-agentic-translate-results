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

use std::ffi::{c_char, c_int};
use std::sync::atomic::{AtomicI32, Ordering};

// ---------------------------------------------------------------------------
// File-scope (`static`) state from lib.c.
//
//     static int global_counter = 0;
//     static int global_accumulator = 0;
//
// These are process-wide mutable state in the C code and therefore persist
// across calls into the shared library.  Atomics with relaxed ordering are used
// so that the Rust translation needs no `unsafe` for the accesses while
// behaving identically to the plain C loads/stores in single-threaded use.
// ---------------------------------------------------------------------------
static GLOBAL_COUNTER: AtomicI32 = AtomicI32::new(0);
static GLOBAL_ACCUMULATOR: AtomicI32 = AtomicI32::new(0);

#[inline]
fn global_counter_get() -> c_int {
    GLOBAL_COUNTER.load(Ordering::Relaxed)
}

#[inline]
fn global_counter_set(v: c_int) {
    GLOBAL_COUNTER.store(v, Ordering::Relaxed);
}

#[inline]
fn global_accumulator_get() -> c_int {
    GLOBAL_ACCUMULATOR.load(Ordering::Relaxed)
}

#[inline]
fn global_accumulator_set(v: c_int) {
    GLOBAL_ACCUMULATOR.store(v, Ordering::Relaxed);
}

// typedef int (*operation_func)(int, int, int);
pub type OperationFunc = extern "C" fn(c_int, c_int, c_int) -> c_int;

// typedef void (*modifier_func)(int, int);
pub type ModifierFunc = extern "C" fn(c_int, c_int);

// ---------------------------------------------------------------------------
// void increment_counter(int value, int unused_param)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    // global_counter += value;
    global_counter_set(global_counter_get().wrapping_add(value));
}

// ---------------------------------------------------------------------------
// void update_accumulator(int value, int unused_param)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    // global_accumulator = global_accumulator * 2 + value;
    global_accumulator_set(
        global_accumulator_get()
            .wrapping_mul(2)
            .wrapping_add(value),
    );
}

// ---------------------------------------------------------------------------
// int apply_operation(operation_func op, int a, int b, int c)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(op: OperationFunc, a: c_int, b: c_int, c: c_int) -> c_int {
    op(a, b, c)
}

// ---------------------------------------------------------------------------
// int add_three(int a, int b, int c)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(c)
}

// ---------------------------------------------------------------------------
// int multiply_add(int a, int b, int c)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

// ---------------------------------------------------------------------------
// int complex_calc(int a, int b, int c)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    // return (a - b) * c + global_counter;
    a.wrapping_sub(b)
        .wrapping_mul(c)
        .wrapping_add(global_counter_get())
}

// ---------------------------------------------------------------------------
// typedef struct {
//     int id;
//     int value;
//     time_t timestamp;
//     char name[32];
// } DataRecord;
//
// x86-64 Linux layout: size 48, align 8, id@0, value@4, timestamp@8, name@16.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: TimeT,
    pub name: [c_char; 32],
}

/// `time_t` as used by the platform C library.
#[cfg(all(target_pointer_width = "64", not(windows)))]
pub type TimeT = i64;
#[cfg(all(target_pointer_width = "32", not(windows)))]
pub type TimeT = i32;
#[cfg(windows)]
pub type TimeT = i64;

/// `time(&t)` — seconds since the Unix epoch.
fn c_time() -> TimeT {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as TimeT,
        Err(e) => -(e.duration().as_secs() as i64) as TimeT,
    }
}

// ---------------------------------------------------------------------------
// void shift_array_data(int *arr, int size, int shift_by)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let shift = shift_by as usize;
        let remaining = (size - shift_by) as usize;
        unsafe {
            // memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int));
            std::ptr::copy(arr.add(shift), arr, remaining);
            // memset(arr + (size - shift_by), 0, shift_by * sizeof(int));
            std::ptr::write_bytes(arr.add(remaining), 0u8, shift);
        }
    }
}

// ---------------------------------------------------------------------------
// int process_pointer_data(int *ptr, int multiplier)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr: *mut c_int, multiplier: c_int) -> c_int {
    let value = unsafe { std::ptr::read(ptr) };
    // return value * multiplier + global_accumulator;
    value
        .wrapping_mul(multiplier)
        .wrapping_add(global_accumulator_get())
}

// ---------------------------------------------------------------------------
// int compute_with_dynamic_memory(int base, int count)
//
// The C version mallocs `count` ints, fills them with `base + i * 3` and sums
// them.  The sum is reproduced with identical wrapping integer arithmetic; for
// count <= 0 no element is ever touched and the result is 0, exactly as in C.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;

    let mut i: c_int = 0;
    while i < count {
        // temp_array[i] = base + i * 3;  then  sum += temp_array[i];
        sum = sum.wrapping_add(base.wrapping_add(i.wrapping_mul(3)));
        i = i.wrapping_add(1);
    }

    sum
}

// ---------------------------------------------------------------------------
// int get_time_based_value(int seed)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let current_time: TimeT = c_time();

    // reference_time = current_time - (seed * 3600);
    // `seed * 3600` is evaluated in `int` (wrapping on the usual targets) and
    // then converted to time_t.
    let reference_time: TimeT = current_time.wrapping_sub(seed.wrapping_mul(3600) as TimeT);

    // double diff = difftime(current_time, reference_time);
    let diff: f64 = (current_time as f64) - (reference_time as f64);

    // return (int)(diff / 100) + seed;
    ((diff / 100.0) as c_int).wrapping_add(seed)
}

// ---------------------------------------------------------------------------
// int manipulate_records(DataRecord *records, int num_records, int shift)
//
// NOTE: the loop bound is `num_records - shift`, which for a non-positive
// `shift` walks past the end of the array.  That out-of-bounds read is part of
// the original behaviour and is reproduced verbatim (no bug fixes).
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        // memmove(records, records + shift,
        //         (num_records - shift) * sizeof(DataRecord));
        unsafe {
            std::ptr::copy(
                records.add(shift as usize),
                records,
                (num_records - shift) as usize,
            );
        }
    }

    let mut i: c_int = 0;
    while i < num_records.wrapping_sub(shift) {
        // total += records[i].value;
        let value = unsafe { std::ptr::read(&raw const (*records.offset(i as isize)).value) };
        total = total.wrapping_add(value);
        i = i.wrapping_add(1);
    }

    total
}

// ---------------------------------------------------------------------------
// int hatch(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
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

    // int *dynamic_data = (int *)malloc(10 * sizeof(int));
    let mut dynamic_data: [c_int; 10] = [0; 10];
    for i in 0..10i32 {
        dynamic_data[i as usize] = param1.wrapping_add(i);
    }

    // result += process_pointer_data(&dynamic_data[5], param2);
    result = result.wrapping_add(unsafe {
        process_pointer_data(dynamic_data.as_mut_ptr().add(5), param2)
    });

    unsafe { shift_array_data(dynamic_data.as_mut_ptr(), 10, 3) };
    result = result.wrapping_add(dynamic_data[0]);

    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = (DataRecord *)malloc(5 * sizeof(DataRecord));
    let mut records: [DataRecord; 5] = [DataRecord {
        id: 0,
        value: 0,
        timestamp: 0,
        name: [0; 32],
    }; 5];

    for i in 0..5i32 {
        let rec = &mut records[i as usize];
        rec.id = i;
        rec.value = param4.wrapping_add(i.wrapping_mul(10));
        rec.timestamp = c_time();
        // snprintf(records[i].name, 32, "Record_%d", i);
        snprintf_record_name(&mut rec.name, i);
    }

    result = result.wrapping_add(unsafe { manipulate_records(records.as_mut_ptr(), 5, 2) });

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    result = result.wrapping_add(
        global_counter_get().wrapping_add(global_accumulator_get()),
    );

    result
}

/// `snprintf(dst, 32, "Record_%d", i)`
fn snprintf_record_name(dst: &mut [c_char; 32], i: c_int) {
    let text = format!("Record_{}", i);
    let bytes = text.as_bytes();
    let n = core::cmp::min(bytes.len(), dst.len() - 1);
    for (slot, &b) in dst[..n].iter_mut().zip(bytes.iter()) {
        *slot = b as c_char;
    }
    dst[n] = 0;
}
