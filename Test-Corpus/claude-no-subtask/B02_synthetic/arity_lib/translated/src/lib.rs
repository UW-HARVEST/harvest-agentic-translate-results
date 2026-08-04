// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_char;
use std::os::raw::{c_int, c_uchar};

#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

fn shift_array(arr: &mut [c_int], size: usize, positions: usize) {
    if positions > 0 && positions < size {
        // memmove(arr + positions, arr, (size - positions) * sizeof(int));
        // i.e. shift values right by `positions`
        unsafe {
            let p = arr.as_mut_ptr();
            std::ptr::copy(p, p.add(positions), size - positions);
        }
        for i in 0..positions {
            arr[i] = 0;
        }
    }
}

fn process_string(s: *const c_char) -> c_int {
    unsafe {
        if *s != 0 {
            // strlen
            let mut len: usize = 0;
            while *s.add(len) != 0 {
                len += 1;
            }
            return len as c_int;
        }
    }
    0
}

fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1: c_int = 0b11110000;
    let mask2: c_int = 0b00001111;
    let mask3: c_int = 0b10101010;
    let mask4: c_int = 0b01010101;

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
    unsafe {
        let ptr1 = libc::malloc(std::mem::size_of::<c_int>()) as *mut c_int;
        let ptr2 = libc::malloc(std::mem::size_of::<c_int>()) as *mut c_int;

        if ptr1.is_null() || ptr2.is_null() {
            libc::free(ptr1 as *mut libc::c_void);
            libc::free(ptr2 as *mut libc::c_void);
            return -1;
        }

        *ptr1 = val1;
        *ptr2 = val2;

        let mut result: c_int = 0;

        if (ptr1 as usize) < (ptr2 as usize) {
            result = 1;
        } else if (ptr1 as usize) > (ptr2 as usize) {
            result = 2;
        } else {
            result = 3;
        }

        let uninit_ptr = ptr1;
        result += if *uninit_ptr > 0 { 10 } else { 0 };

        libc::free(ptr1 as *mut libc::c_void);
        libc::free(ptr2 as *mut libc::c_void);

        result
    }
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    // char test_str[] = "Hello"; — 6 bytes incl. NUL
    let test_str: [c_char; 6] = [
        b'H' as c_char,
        b'e' as c_char,
        b'l' as c_char,
        b'l' as c_char,
        b'o' as c_char,
        0,
    ];
    let empty_str: [c_char; 1] = [0];

    let len1 = process_string(test_str.as_ptr());
    let len2 = process_string(empty_str.as_ptr());

    result += len1 + len2;

    shift_array(&mut block.values, 4, 1);

    for i in 0..(block.count as usize) {
        result += block.values[i];
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix: [[c_int; 4]; 3] = [[0; 4]; 3];
    init_matrix(&mut matrix);

    result += matrix[0][0] + matrix[2][3];

    let alloc_result = compare_allocations(param1, param2);
    result += alloc_result;

    if param3 != 0 {
        result = (result * param3) / 100;
    }

    if param4 != 0 {
        result += param4;
    }

    result
}

fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_uchar, params: *mut c_int) -> c_int {
    if len < 2 {
        -1
    } else if len == 2 {
        arity2(*params, *params.add(1))
    } else if len == 3 {
        arity3(*params, *params.add(1), *params.add(2))
    } else {
        arity4(
            *params,
            *params.add(1),
            *params.add(2),
            *params.add(3),
        )
    }
}
