// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// Rust translation of c_src/src/lib.c. Preserves the original C
// behavior (including any quirks) for byte-identical output.

use std::ffi::{c_char, c_int, c_uchar};
use std::ptr;

#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    if positions > 0 && positions < size {
        // memmove(arr + positions, arr, (size - positions) * sizeof(int));
        let count = (size - positions) as usize;
        // SAFETY: caller is responsible for providing a buffer of `size` ints.
        unsafe {
            ptr::copy(arr, arr.add(positions as usize), count);
            for i in 0..(positions as usize) {
                *arr.add(i) = 0;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(s: *const c_char) -> c_int {
    // if (*str) { return (int)strlen(str); } return 0;
    unsafe {
        if *s != 0 {
            // strlen
            let mut len: usize = 0;
            while *s.add(len) != 0 {
                len += 1;
            }
            len as c_int
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut [c_int; 4]) {
    let temp: [[c_int; 4]; 3] = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ];

    // SAFETY: caller provides a 3x4 matrix.
    unsafe {
        for i in 0..3 {
            for j in 0..4 {
                (*matrix.add(i))[j] = temp[i][j];
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    // Use libc malloc/free so address-comparison behavior matches the C version.
    let size = std::mem::size_of::<c_int>();

    // SAFETY: standard malloc/free usage, matching the C implementation.
    unsafe {
        let ptr1 = libc::malloc(size) as *mut c_int;
        let ptr2 = libc::malloc(size) as *mut c_int;

        if ptr1.is_null() || ptr2.is_null() {
            libc::free(ptr1 as *mut libc::c_void);
            libc::free(ptr2 as *mut libc::c_void);
            return -1;
        }

        *ptr1 = val1;
        *ptr2 = val2;

        let mut result: c_int;

        if (ptr1 as usize) < (ptr2 as usize) {
            result = 1;
        } else if (ptr1 as usize) > (ptr2 as usize) {
            result = 2;
        } else {
            result = 3;
        }

        // uninit_ptr = ptr1; result += (*uninit_ptr > 0) ? 10 : 0;
        let uninit_ptr = ptr1;
        result += if *uninit_ptr > 0 { 10 } else { 0 };

        libc::free(ptr1 as *mut libc::c_void);
        libc::free(ptr2 as *mut libc::c_void);

        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: ptr::null_mut(),
    };

    // char test_str[] = "Hello";
    let test_str: [c_char; 6] = [b'H' as c_char, b'e' as c_char, b'l' as c_char, b'l' as c_char, b'o' as c_char, 0];
    // char empty_str[] = "";
    let empty_str: [c_char; 1] = [0];

    // SAFETY: pointers are to local NUL-terminated buffers.
    let len1 = unsafe { process_string(test_str.as_ptr()) };
    let len2 = unsafe { process_string(empty_str.as_ptr()) };

    result += len1 + len2;

    // SAFETY: shifting in-place on a 4-element local array.
    unsafe {
        shift_array(block.values.as_mut_ptr(), 4, 1);
    }

    for i in 0..(block.count as usize) {
        result += block.values[i];
    }

    result = apply_bitmask(result, param1.rem_euclid_c(4));

    let mut matrix: [[c_int; 4]; 3] = [[0; 4]; 3];
    // SAFETY: passing a valid 3x4 matrix.
    unsafe {
        init_matrix(matrix.as_mut_ptr());
    }

    result += matrix[0][0] + matrix[2][3];

    let alloc_result = compare_allocations(param1, param2);
    result += alloc_result;

    if param3 != 0 {
        // wrapping arithmetic to match C signed int behavior on overflow
        result = result.wrapping_mul(param3) / 100;
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    result
}

// Helper trait providing C-style `%` semantics for c_int (truncating toward zero).
trait CRem {
    fn rem_euclid_c(self, rhs: Self) -> Self;
}

impl CRem for c_int {
    #[inline]
    fn rem_euclid_c(self, rhs: c_int) -> c_int {
        // C's `%` truncates toward zero, which matches Rust's `%` for i32.
        self % rhs
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

// NOTE: the public header declares `int arity(int len, int *params);` but the
// C implementation actually defines `int arity(unsigned char len, int *params)`.
// The linker symbol is `arity`, and the implementation reads only the low 8 bits
// of the first argument. We preserve the implementation signature exactly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_uchar, params: *mut c_int) -> c_int {
    if len < 2 {
        -1
    } else if len == 2 {
        // SAFETY: caller is responsible for providing a buffer of at least `len` ints.
        unsafe { arity2(*params, *params.add(1)) }
    } else if len == 3 {
        unsafe { arity3(*params, *params.add(1), *params.add(2)) }
    } else {
        unsafe { arity4(*params, *params.add(1), *params.add(2), *params.add(3)) }
    }
}
