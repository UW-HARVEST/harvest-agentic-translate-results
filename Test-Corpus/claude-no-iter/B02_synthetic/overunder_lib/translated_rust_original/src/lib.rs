// Copyright 2025 MIT Lincoln Laboratory
// Rust translation preserving byte-identical output of the original C code.

use std::ffi::c_char;
use std::ffi::c_double;
use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DataBlock {
    id: c_int,
    value: c_double,
    label: [c_char; 20],
}

fn safe_double_to_int(d: f64) -> c_int {
    if d > c_int::MAX as f64 {
        c_int::MAX
    } else if d < c_int::MIN as f64 {
        c_int::MIN
    } else if d.is_nan() {
        0
    } else {
        // Match C cast semantics: truncation toward zero.
        d as c_int
    }
}

fn process_with_fallthrough(code: c_int, base_value: c_int) -> c_int {
    let mut result: c_int = base_value;

    // Reproduce C switch fall-through behavior exactly.
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

fn copy_data_block(dest: &mut DataBlock, src: &DataBlock) {
    *dest = *src;
}

fn handle_pointer_operations(value: c_int) -> c_int {
    let local_value: c_int = value.wrapping_mul(2);
    let ptr: *const c_int = &local_value;

    // SAFETY: ptr points to a valid local stack variable.
    let result = unsafe { *ptr }.wrapping_add(100);

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn overunder(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    let mut total: c_int = 0;

    let result_1: c_int = a;
    let result_2: c_int = b;
    let _result_3: c_int = c;
    let _result_4: c_int = d;

    // PRINT_VAR(result_1);
    unsafe {
        printf(b"result_1 = %d\n\0".as_ptr() as *const c_char, result_1);
    }
    // PRINT_VAR(result_2);
    unsafe {
        printf(b"result_2 = %d\n\0".as_ptr() as *const c_char, result_2);
    }

    let temp1: f64 = (a as f64) * 1.5;
    let temp2: f64 = (b as f64) * 2.7;
    let temp3: f64 = (c as f64) / 3.3;
    // d * d + a * a uses C signed int arithmetic; emulate with wrapping ops,
    // then convert to double, matching the C expression order.
    let dd = d.wrapping_mul(d);
    let aa = a.wrapping_mul(a);
    let sum = dd.wrapping_add(aa);
    let temp4: f64 = (sum as f64).sqrt();

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

    // a % 6 with C truncated-division semantics matches Rust's % for i32.
    let switch_result = process_with_fallthrough(a % 6, b);
    unsafe {
        printf(
            b"Switch fall-through result: %d\n\0".as_ptr() as *const c_char,
            switch_result,
        );
    }

    let mut source_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    source_block.id = a;
    source_block.value = temp1;
    // strncpy(source_block.label, "Source", sizeof(source_block.label) - 1);
    let src_bytes = b"Source";
    let n = src_bytes.len().min(source_block.label.len() - 1);
    for i in 0..n {
        source_block.label[i] = src_bytes[i] as c_char;
    }
    // strncpy zero-pads up to (size - 1); subsequent bytes were already 0
    // from initialization, but emulate strncpy anyway.
    for i in n..(source_block.label.len() - 1) {
        source_block.label[i] = 0;
    }
    // Force the final terminator (matches the explicit C assignment).
    let last = source_block.label.len() - 1;
    source_block.label[last] = 0;

    let mut dest_block = DataBlock {
        id: 0,
        value: 0.0,
        label: [0; 20],
    };
    copy_data_block(&mut dest_block, &source_block);

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

    total = total
        .wrapping_add(conv1)
        .wrapping_add(conv2)
        .wrapping_add(conv3)
        .wrapping_add(conv4)
        .wrapping_add(switch_result)
        .wrapping_add(ptr_result);
    total = total.wrapping_add(dest_block.id);

    let overflow_test: f64 = 1e15;
    let safe_conv = safe_double_to_int(overflow_test);
    unsafe {
        printf(
            b"Overflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv,
        );
    }

    let underflow_test: f64 = -1e15;
    let safe_conv2 = safe_double_to_int(underflow_test);
    unsafe {
        printf(
            b"Underflow protected conversion: %d\n\0".as_ptr() as *const c_char,
            safe_conv2,
        );
    }

    let array1: [c_int; 5] = [a, b, c, d, a.wrapping_add(b)];
    let mut array2: [c_int; 5] = [0; 5];
    array2.copy_from_slice(&array1);

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
