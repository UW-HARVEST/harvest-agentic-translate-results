// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::os::raw::{c_char, c_int, c_uchar};

#[allow(dead_code)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

fn shift_array(arr: &mut [c_int], size: usize, positions: usize) {
    if positions > 0 && positions < size {
        // Mimic memmove(arr + positions, arr, (size - positions) * sizeof(int));
        // Move elements [0..size-positions) to [positions..size)
        for i in (0..(size - positions)).rev() {
            arr[i + positions] = arr[i];
        }
        for i in 0..positions {
            arr[i] = 0;
        }
    }
}

fn process_string(s: &[u8]) -> c_int {
    // s should be a null-terminated byte slice
    if !s.is_empty() && s[0] != 0 {
        // strlen behavior: count bytes until null terminator
        let mut len: c_int = 0;
        for &b in s.iter() {
            if b == 0 {
                break;
            }
            len += 1;
        }
        return len;
    }
    0
}

fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1: c_int = 0b1111_0000;
    let mask2: c_int = 0b0000_1111;
    let mask3: c_int = 0b1010_1010;
    let mask4: c_int = 0b0101_0101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

fn init_matrix(matrix: &mut [[c_int; 4]; 3]) {
    let temp: [[c_int; 4]; 3] = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ];

    for i in 0..3 {
        for j in 0..4 {
            matrix[i][j] = temp[i][j];
        }
    }
}

fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    // Allocate two integers on the heap (mimicking malloc)
    let mut ptr1 = Box::new(0i32);
    let mut ptr2 = Box::new(0i32);

    *ptr1 = val1;
    *ptr2 = val2;

    let mut result: c_int = 0;

    let p1_addr = (&*ptr1) as *const c_int as usize;
    let p2_addr = (&*ptr2) as *const c_int as usize;

    if p1_addr < p2_addr {
        result = 1;
    } else if p1_addr > p2_addr {
        result = 2;
    } else {
        result = 3;
    }

    // uninit_ptr = ptr1; result += (*uninit_ptr > 0) ? 10 : 0;
    let uninit_ptr_val = *ptr1;
    result += if uninit_ptr_val > 0 { 10 } else { 0 };

    // ptr1, ptr2 dropped automatically (free)
    drop(ptr1);
    drop(ptr2);

    result
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    let test_str: &[u8] = b"Hello\0";
    let empty_str: &[u8] = b"\0";

    let len1 = process_string(test_str);
    let len2 = process_string(empty_str);

    result += len1 + len2;

    shift_array(&mut block.values, 4, 1);

    for i in 0..(block.count as usize) {
        result = result.wrapping_add(block.values[i]);
    }

    result = apply_bitmask(result, param1.rem_euclid(4));

    let mut matrix: [[c_int; 4]; 3] = [[0; 4]; 3];
    init_matrix(&mut matrix);

    result = result.wrapping_add(matrix[0][0]).wrapping_add(matrix[2][3]);

    let alloc_result = compare_allocations(param1, param2);
    result = result.wrapping_add(alloc_result);

    if param3 != 0 {
        result = result.wrapping_mul(param3) / 100;
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    result
}

fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

/// # Safety
/// `params` must point to at least `len` valid `c_int` values when `len >= 2`.
#[no_mangle]
pub unsafe extern "C" fn arity(len: c_uchar, params: *const c_int) -> c_int {
    if len < 2 {
        return -1;
    } else if len == 2 {
        return arity2(*params.offset(0), *params.offset(1));
    } else if len == 3 {
        return arity3(*params.offset(0), *params.offset(1), *params.offset(2));
    } else {
        return arity4(
            *params.offset(0),
            *params.offset(1),
            *params.offset(2),
            *params.offset(3),
        );
    }
}
