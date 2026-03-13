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

use std::ffi::c_int;

extern "C" {
    fn time(t: *mut i64) -> i64;
    fn difftime(time1: i64, time0: i64) -> f64;
}

static mut GLOBAL_COUNTER: c_int = 0;
static mut GLOBAL_ACCUMULATOR: c_int = 0;

fn increment_counter(value: c_int, _unused: c_int) {
    unsafe {
        GLOBAL_COUNTER += value;
    }
}

fn update_accumulator(value: c_int, _unused: c_int) {
    unsafe {
        GLOBAL_ACCUMULATOR = GLOBAL_ACCUMULATOR.wrapping_mul(2).wrapping_add(value);
    }
}

fn add_three(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_add(b).wrapping_add(c)
}

fn multiply_add(a: c_int, b: c_int, c: c_int) -> c_int {
    a.wrapping_mul(b).wrapping_add(c)
}

fn complex_calc(a: c_int, b: c_int, c: c_int) -> c_int {
    unsafe { a.wrapping_sub(b).wrapping_mul(c).wrapping_add(GLOBAL_COUNTER) }
}

fn shift_array_data(arr: &mut [c_int], shift_by: usize) {
    let size = arr.len();
    if shift_by > 0 && shift_by < size {
        arr.copy_within(shift_by.., 0);
        for i in (size - shift_by)..size {
            arr[i] = 0;
        }
    }
}

fn process_pointer_data(value: c_int, multiplier: c_int) -> c_int {
    unsafe { value.wrapping_mul(multiplier).wrapping_add(GLOBAL_ACCUMULATOR) }
}

fn compute_with_dynamic_memory(base: c_int, count: c_int) -> c_int {
    let count = count as usize;
    let mut sum: c_int = 0;
    for i in 0..count {
        sum = sum.wrapping_add(base.wrapping_add((i as c_int).wrapping_mul(3)));
    }
    sum
}

fn get_time_based_value(seed: c_int) -> c_int {
    unsafe {
        let mut current_time: i64 = 0;
        time(&mut current_time);
        let reference_time = current_time - (seed as i64) * 3600;
        let diff = difftime(current_time, reference_time);
        (diff / 100.0) as c_int + seed
    }
}

#[repr(C)]
struct DataRecord {
    id: c_int,
    value: c_int,
    timestamp: i64,
    name: [u8; 32],
}

fn manipulate_records(records: &mut [DataRecord], shift: usize) -> c_int {
    let num = records.len();
    if shift > 0 && shift < num {
        // memmove via rotate
        records.rotate_left(shift);
    }
    let mut total: c_int = 0;
    for i in 0..(num - shift) {
        total = total.wrapping_add(records[i].value);
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn hatch(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    // modifier_func = increment_counter; mod_func(param1, 999);
    increment_counter(param1, 999);

    // modifier_func = update_accumulator; mod_func(param2, 888);
    update_accumulator(param2, 888);

    // op_func = add_three
    result = result.wrapping_add(add_three(param1, param2, param3));

    // op_func = multiply_add
    result = result.wrapping_add(multiply_add(param2, param3, param4));

    // op_func = complex_calc
    result = result.wrapping_add(complex_calc(param1, param3, param4));

    // dynamic_data[10], filled with param1+i
    let mut dynamic_data = vec![0i32; 10];
    for i in 0..10 {
        dynamic_data[i] = param1.wrapping_add(i as c_int);
    }

    result = result.wrapping_add(process_pointer_data(dynamic_data[5], param2));

    shift_array_data(&mut dynamic_data, 3);
    result = result.wrapping_add(dynamic_data[0]);

    result = result.wrapping_add(get_time_based_value(param3));

    // DataRecord records[5]
    let mut records: Vec<DataRecord> = Vec::with_capacity(5);
    for i in 0..5i32 {
        let mut name = [0u8; 32];
        let s = format!("Record_{}", i);
        let bytes = s.as_bytes();
        let len = bytes.len().min(31);
        name[..len].copy_from_slice(&bytes[..len]);
        // name[len] already 0 from initialization

        let mut ts: i64 = 0;
        unsafe {
            time(&mut ts);
        }

        records.push(DataRecord {
            id: i,
            value: param4.wrapping_add(i.wrapping_mul(10)),
            timestamp: ts,
            name,
        });
    }

    result = result.wrapping_add(manipulate_records(&mut records, 2));

    result = result.wrapping_add(compute_with_dynamic_memory(param1, 8));

    unsafe {
        result = result.wrapping_add(GLOBAL_COUNTER.wrapping_add(GLOBAL_ACCUMULATOR));
    }

    result
}
