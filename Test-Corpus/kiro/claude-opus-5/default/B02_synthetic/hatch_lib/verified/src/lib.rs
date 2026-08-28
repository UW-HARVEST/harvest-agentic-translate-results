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

/// `time_t` on Linux / x86_64 (and every other 64-bit glibc target).
type TimeT = i64;

// ---------------------------------------------------------------------------
// File-scope mutable state (the C `static int` globals).
//
// The C code is not thread safe; `static mut` reproduces the exact same
// semantics (shared, unsynchronised, persists across calls).
// ---------------------------------------------------------------------------
static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

#[inline]
fn get_counter() -> c_int {
    unsafe { GLOBAL_COUNTER }
}

#[inline]
fn set_counter(v: c_int) {
    unsafe { GLOBAL_COUNTER = v };
}

#[inline]
fn get_accumulator() -> c_int {
    unsafe { GLOBAL_ACCUMULATOR }
}

#[inline]
fn set_accumulator(v: c_int) {
    unsafe { GLOBAL_ACCUMULATOR = v };
}

// ---------------------------------------------------------------------------
// Function pointer typedefs
// ---------------------------------------------------------------------------
pub type OperationFunc = extern "C" fn(c_int, c_int, c_int) -> c_int;
pub type ModifierFunc = extern "C" fn(c_int, c_int);

// ---------------------------------------------------------------------------
// void increment_counter(int value, int unused_param)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int, _unused_param: c_int) {
    // C: global_counter += value;  (wrapping reproduces the two's-complement
    // result produced by every real compiler for signed overflow)
    set_counter(get_counter().wrapping_add(value));
}

// ---------------------------------------------------------------------------
// void update_accumulator(int value, int unused_param)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn update_accumulator(value: c_int, _unused_param: c_int) {
    // C: global_accumulator = global_accumulator * 2 + value;
    set_accumulator(get_accumulator().wrapping_mul(2).wrapping_add(value));
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
    a.wrapping_sub(b)
        .wrapping_mul(c)
        .wrapping_add(get_counter())
}

// ---------------------------------------------------------------------------
// typedef struct { int id; int value; time_t timestamp; char name[32]; }
// DataRecord;
//
// Layout on LP64: id @0, value @4, timestamp @8, name @16..48, size 48.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataRecord {
    pub id: c_int,
    pub value: c_int,
    pub timestamp: TimeT,
    pub name: [c_char; 32],
}

// ---------------------------------------------------------------------------
// void shift_array_data(int *arr, int size, int shift_by)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array_data(arr: *mut c_int, size: c_int, shift_by: c_int) {
    if shift_by > 0 && shift_by < size {
        let n = (size - shift_by) as usize;
        // memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int));
        std::ptr::copy(arr.add(shift_by as usize), arr, n);
        // memset(arr + (size - shift_by), 0, shift_by * sizeof(int));
        std::ptr::write_bytes(arr.add(n), 0u8, shift_by as usize);
    }
}

// ---------------------------------------------------------------------------
// int process_pointer_data(int *ptr, int multiplier)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_pointer_data(ptr: *const c_int, multiplier: c_int) -> c_int {
    let value = *ptr;
    value.wrapping_mul(multiplier).wrapping_add(get_accumulator())
}

// ---------------------------------------------------------------------------
// int compute_with_dynamic_memory(int base, int count)
//
// In C, `malloc(count * sizeof(int))` with a negative `count` computes a huge
// size_t and returns NULL, but the following loops never execute (i < count is
// immediately false), so nothing is dereferenced and the result is 0.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    if count <= 0 {
        return 0;
    }

    let n = count as usize;
    let mut temp_array: Vec<c_int> = Vec::with_capacity(n);

    for i in 0..count {
        temp_array.push(base.wrapping_add(i.wrapping_mul(3)));
    }

    let mut sum: c_int = 0;
    for i in 0..n {
        sum = sum.wrapping_add(temp_array[i]);
    }

    sum
}

// ---------------------------------------------------------------------------
// int get_time_based_value(int seed)
//
// `reference_time` is derived from `current_time`, so `difftime` cancels the
// wall clock out entirely: the result only depends on `seed`.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn get_time_based_value(seed: c_int) -> c_int {
    let current_time: TimeT = current_time_secs();

    // reference_time = current_time - (seed * 3600);
    // `seed * 3600` is int arithmetic, then converted to time_t.
    let offset: TimeT = seed.wrapping_mul(3600) as TimeT;
    let reference_time: TimeT = current_time.wrapping_sub(offset);

    // double diff = difftime(current_time, reference_time);
    let diff: f64 = (current_time as f64) - (reference_time as f64);

    // return (int)(diff / 100) + seed;
    ((diff / 100.0) as c_int).wrapping_add(seed)
}

// ---------------------------------------------------------------------------
// int manipulate_records(DataRecord *records, int num_records, int shift)
//
// Note: the trailing loop bound is `num_records - shift` regardless of whether
// the memmove happened, exactly as in the C. Reproduced verbatim (including
// the out-of-bounds read when `shift < 0`).
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn manipulate_records(
    records: *mut DataRecord,
    num_records: c_int,
    shift: c_int,
) -> c_int {
    let mut total: c_int = 0;

    if shift > 0 && shift < num_records {
        std::ptr::copy(
            records.add(shift as usize),
            records,
            (num_records - shift) as usize,
        );
    }

    let mut i: c_int = 0;
    while i < num_records.wrapping_sub(shift) {
        total = total.wrapping_add((*records.offset(i as isize)).value);
        i += 1;
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

    // int *dynamic_data = malloc(10 * sizeof(int));
    let mut dynamic_data: Vec<c_int> = Vec::with_capacity(10);
    for i in 0..10i32 {
        dynamic_data.push(param1.wrapping_add(i));
    }

    unsafe {
        result = result.wrapping_add(process_pointer_data(
            dynamic_data.as_ptr().add(5),
            param2,
        ));

        shift_array_data(dynamic_data.as_mut_ptr(), 10, 3);
    }
    result = result.wrapping_add(dynamic_data[0]);

    drop(dynamic_data);

    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord *records = malloc(5 * sizeof(DataRecord));
    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5i32 {
        let mut rec = DataRecord {
            id: i,
            value: param4.wrapping_add(i.wrapping_mul(10)),
            timestamp: 0,
            name: [0; 32],
        };
        rec.timestamp = current_time_secs();
        // snprintf(records[i].name, 32, "Record_%d", i);
        write_snprintf(&mut rec.name, &format!("Record_{}", i));
        records.push(rec);
    }

    result = result.wrapping_add(unsafe { manipulate_records(records.as_mut_ptr(), 5, 2) });

    drop(records);

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    result = result
        .wrapping_add(get_counter())
        .wrapping_add(get_accumulator());

    result
}

// ---------------------------------------------------------------------------
// Helpers (not part of the C API)
// ---------------------------------------------------------------------------

/// Equivalent of `time(NULL)`: seconds since the Unix epoch.
fn current_time_secs() -> TimeT {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs() as TimeT,
        Err(e) => -(e.duration().as_secs() as TimeT),
    }
}

/// Equivalent of `snprintf(dst, dst.len(), "%s", src)`: copy as many bytes as
/// fit, always NUL-terminating.
fn write_snprintf(dst: &mut [c_char], src: &str) {
    if dst.is_empty() {
        return;
    }
    let bytes = src.as_bytes();
    let max = dst.len() - 1;
    let n = if bytes.len() < max { bytes.len() } else { max };
    for (slot, &b) in dst.iter_mut().zip(&bytes[..n]) {
        *slot = b as c_char;
    }
    dst[n] = 0;
}
