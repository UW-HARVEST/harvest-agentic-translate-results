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

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::c_int;

// extern "C" function-pointer matching the C typedef:
//   typedef int (*operation_func)(int a, int b, int unused1, int unused2);
type OperationFunc = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Result_ {
    pub value: c_int,
    pub scaled: f64,
    pub rank: c_int,
}

#[repr(C)]
pub struct ResultArray {
    pub data: [Result_; 10],
    pub count: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused1: c_int, _unused2: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(
    a: c_int,
    b: c_int,
    _unused1: c_int,
    _unused2: c_int,
) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(
    a: c_int,
    b: c_int,
    _unused1: c_int,
    _unused2: c_int,
) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(
    a: c_int,
    b: c_int,
    _unused1: c_int,
    _unused2: c_int,
) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: f64) -> c_int {
    if d >= i32::MAX as f64 {
        return i32::MAX;
    }
    if d <= i32::MIN as f64 {
        return i32::MIN;
    }
    if d != d {
        return 0;
    }
    d as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn compute_scaled_value(base: c_int, scale_factor: f64) -> c_int {
    let scaled = base as f64 * scale_factor;
    safe_double_to_int(scaled)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_results_in_array(
    arr: *mut ResultArray,
    idx1: c_int,
    idx2: c_int,
) -> c_int {
    let arr = &*arr;
    if idx1 >= arr.count || idx2 >= arr.count {
        return 0;
    }
    // Pointer comparison within the same array: equivalent to comparing indices.
    if idx1 < idx2 {
        -1
    } else if idx1 > idx2 {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_result_array(
    arr: *mut ResultArray,
    values: *mut c_int,
    count: c_int,
) {
    let arr = &mut *arr;
    arr.count = if count < 10 { count } else { 10 };

    for i in 0..arr.count as usize {
        let v = *values.add(i);
        arr.data[i] = Result_ {
            value: v,
            scaled: v as f64 * 1.5,
            rank: i as c_int,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_with_foreach(
    arr: *mut ResultArray,
    op: OperationFunc,
) -> c_int {
    let arr = &mut *arr;
    let mut total: c_int = 0;
    let count = arr.count as usize;

    for i in 0..count {
        let item_value = arr.data[i].value;
        let item_rank = arr.data[i].rank;
        let result = op(item_value, item_rank, 0, 0);
        total = total.wrapping_add(result);

        let temp = result as f64 * 0.75;
        arr.data[i].scaled = temp;
        arr.data[i].value = safe_double_to_int(temp);
    }

    total
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compute_weighted_sum(arr: *mut ResultArray) -> c_int {
    let arr = &mut *arr;
    let mut sum: c_int = 0;

    for i in 0..arr.count as usize {
        // current = &arr->data[i], base = &arr->data[0]
        // weight = (current > base) ? (int)(current - base) : 1
        let weight: c_int = if i > 0 { i as c_int } else { 1 };

        let weighted = arr.data[i].value as f64 * weight as f64 * 0.8;
        sum = sum.wrapping_add(safe_double_to_int(weighted));
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn arrayfunc(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let operations: [OperationFunc; 4] = [
        add_operation,
        multiply_operation,
        subtract_operation,
        modulo_operation,
    ];

    let mut values: [c_int; 8] = [
        param1,
        param2,
        param3,
        param4,
        param1.wrapping_add(param2),
        param2.wrapping_sub(param3),
        param3.wrapping_mul(2),
        // param4 / 2 + 1: signed division truncated toward zero.
        (param4 / 2).wrapping_add(1),
    ];

    let mut arr = ResultArray {
        data: [Result_ {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
        count: 0,
    };
    unsafe {
        init_result_array(&mut arr as *mut ResultArray, values.as_mut_ptr(), 8);
    }

    let mut result: c_int = 0;

    for i in 0..4 {
        result = result.wrapping_add(unsafe {
            process_with_foreach(&mut arr as *mut ResultArray, operations[i])
        });
    }

    result = result.wrapping_add(unsafe { compute_weighted_sum(&mut arr as *mut ResultArray) });

    for i in 0..(arr.count - 1) {
        let cmp = unsafe { compare_results_in_array(&mut arr as *mut ResultArray, i, i + 1) };
        result = result.wrapping_add(cmp);
    }

    let final_scale = result as f64 * 0.333;
    safe_double_to_int(final_scale)
}
