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

use std::ffi::{c_char, c_double, c_int};

// The original C code emits all of its output through libc's `printf`. Calling
// the same function here keeps the formatting (`%.2f` rounding, etc.) and the
// stdout buffering behaviour byte-for-byte identical to the C build.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `typedef struct { int id; double value; char label[20]; } DataBlock;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

impl DataBlock {
    /// Mirrors an uninitialized C stack struct closely enough for the observable
    /// behaviour of this program (every field read is written first).
    const fn zeroed() -> Self {
        DataBlock {
            id: 0,
            value: 0.0,
            label: [0; 20],
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    // Order of the checks is preserved exactly: the NaN test comes *after* the
    // range tests (both of which are false for NaN, so NaN still yields 0).
    if d > c_int::MAX as c_double {
        c_int::MAX
    } else if d < c_int::MIN as c_double {
        c_int::MIN
    } else if d.is_nan() {
        0
    } else {
        // The two range checks above bound `d` within [INT_MIN, INT_MAX], so
        // this truncating cast matches C's `(int)d`.
        d as c_int
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result = base_value;

    // Reproduces the C switch statement's fall-through chain.
    match code {
        5 => {
            result = result.wrapping_add(50);
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        4 => {
            result = result.wrapping_add(40);
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        3 => {
            result = result.wrapping_add(30);
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        2 => {
            result = result.wrapping_add(20);
            result = result.wrapping_add(10);
        }
        1 => {
            result = result.wrapping_add(10);
        }
        0 => {
            result = 0;
        }
        _ => {
            result = -1;
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_data_block(dest: *mut DataBlock, src: *const DataBlock) {
    unsafe {
        std::ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, size_of::<DataBlock>());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value = value.wrapping_mul(2);
    let ptr = &local_value;
    (*ptr).wrapping_add(100)
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int;

    let result_1 = a;
    let result_2 = b;
    let _result_3 = c;
    let _result_4 = d;

    unsafe {
        printf(b"result_1 = %d\n\0".as_ptr() as *const c_char, result_1);
        printf(b"result_2 = %d\n\0".as_ptr() as *const c_char, result_2);
    }

    let temp1 = a as c_double * 1.5;
    let temp2 = b as c_double * 2.7;
    let temp3 = c as c_double / 3.3;
    // `d * d + a * a` is evaluated in `int` first (wrapping on overflow, as the
    // native C build does), then converted to double. A negative sum therefore
    // yields NaN from sqrt, which safe_double_to_int maps to 0.
    let temp4 = ((d.wrapping_mul(d)).wrapping_add(a.wrapping_mul(a)) as c_double).sqrt();

    let conv1 = safe_double_to_int(temp1);
    let conv2 = safe_double_to_int(temp2);
    let conv3 = safe_double_to_int(temp3);
    let conv4 = safe_double_to_int(temp4);

    unsafe {
        printf(
            b"Converted values: %d, %d, %d, %d\n\0".as_ptr() as *const c_char,
            conv1,
            conv2,
            conv3,
            conv4,
        );
    }

    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(
            b"Switch fall-through result: %d\n\0".as_ptr() as *const c_char,
            switch_result,
        );
    }

    let mut source_block = DataBlock::zeroed();
    source_block.id = a;
    source_block.value = temp1;
    // strncpy(label, "Source", 19) copies the string and zero-fills the rest.
    strncpy_into(&mut source_block.label, b"Source", 19);
    source_block.label[19] = 0;

    let mut dest_block = DataBlock::zeroed();
    unsafe {
        copy_data_block(&mut dest_block, &source_block);
    }

    unsafe {
        printf(
            b"Copied block: id=%d, value=%.2f, label=%s\n\0".as_ptr() as *const c_char,
            dest_block.id,
            dest_block.value,
            dest_block.label.as_ptr(),
        );
    }

    let ptr_result = handle_pointer_operations(c);
    unsafe {
        printf(
            b"Pointer operation result: %d\n\0".as_ptr() as *const c_char,
            ptr_result,
        );
    }

    total = conv1
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: c_double = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        printf(
            b"Overflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv,
        );
    }

    let underflow_test: c_double = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(
            b"Underflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv2,
        );
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let array2: [c_int; 5] = array1; // memcpy of the whole array

    unsafe {
        printf(b"Array copied via memcpy: \0".as_ptr() as *const c_char);
    }
    for i in 0..5 {
        unsafe {
            printf(b"%d \0".as_ptr() as *const c_char, array2[i]);
        }
        total = total.wrapping_add(array2[i]);
    }
    unsafe {
        printf(b"\n\0".as_ptr() as *const c_char);
    }

    total
}

/// Equivalent of `strncpy(dest, src, n)` for a fixed-size buffer: copies at most
/// `n` bytes from `src` and zero-pads the remainder of those `n` bytes.
fn strncpy_into(dest: &mut [c_char; 20], src: &[u8], n: usize) {
    for i in 0..n {
        dest[i] = if i < src.len() { src[i] as c_char } else { 0 };
    }
}
