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

//! Rust translation of `c_src/src/lib.c`.
//!
//! Behavioral notes (bugs and quirks of the original are reproduced, not fixed):
//!
//! * `compare_allocations` branches on the *relative addresses* returned by two
//!   independent `malloc` calls, then reads back through a pointer the original
//!   code names `uninit_ptr` (it is in fact assigned `ptr1` before use, so the
//!   read is well defined and yields `val1`). The real allocator is used here so
//!   the address comparison resolves the same way it does in C.
//! * `arity` is declared as `int arity(int len, int *params)` in `include/lib.h`
//!   but *defined* as taking `unsigned char len`. The definition wins, so the
//!   length argument is truncated to its low 8 bits.
//! * Signed arithmetic that can overflow uses wrapping operations, matching the
//!   two's-complement wraparound produced by the C compilers in practice.

use std::ffi::{c_char, c_int, c_uchar, c_void};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// ```c
/// typedef struct {
///     int values[4];
///     int count;
///     char *label;
/// } DataBlock;
/// ```
#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    #[allow(dead_code)]
    label: *mut c_char,
}

/// `void shift_array(int *arr, int size, int positions)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    if positions > 0 && positions < size {
        // memmove(arr + positions, arr, (size - positions) * sizeof(int));
        let n = (size - positions) as usize;
        unsafe {
            std::ptr::copy(arr, arr.add(positions as usize), n);
        }
        for i in 0..positions as usize {
            unsafe {
                *arr.add(i) = 0;
            }
        }
    }
}

/// `int process_string(const char *str)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str: *const c_char) -> c_int {
    if unsafe { *str } != 0 {
        let mut len: usize = 0;
        while unsafe { *str.add(len) } != 0 {
            len += 1;
        }
        return len as c_int;
    }
    0
}

/// `int apply_bitmask(int value, int operation)`
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

/// `void init_matrix(int matrix[3][4])`
///
/// The array parameter decays to `int (*)[4]` in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut [c_int; 4]) {
    let temp: [[c_int; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];

    for i in 0..3usize {
        for j in 0..4usize {
            unsafe {
                (*matrix.add(i))[j] = temp[i][j];
            }
        }
    }
}

/// `int compare_allocations(int val1, int val2)`
#[unsafe(no_mangle)]
pub extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = unsafe { malloc(std::mem::size_of::<c_int>()) } as *mut c_int;
    let ptr2 = unsafe { malloc(std::mem::size_of::<c_int>()) } as *mut c_int;

    let uninit_ptr: *mut c_int;

    if ptr1.is_null() || ptr2.is_null() {
        unsafe {
            free(ptr1 as *mut c_void);
            free(ptr2 as *mut c_void);
        }
        return -1;
    }

    unsafe {
        *ptr1 = val1;
        *ptr2 = val2;
    }

    let mut result: c_int;

    if ptr1 < ptr2 {
        result = 1;
    } else if ptr1 > ptr2 {
        result = 2;
    } else {
        result = 3;
    }

    uninit_ptr = ptr1;
    result += if unsafe { *uninit_ptr } > 0 { 10 } else { 0 };

    unsafe {
        free(ptr1 as *mut c_void);
        free(ptr2 as *mut c_void);
    }

    result
}

/// `int arity4(int param1, int param2, int param3, int param4)`
#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    let test_str: [c_char; 6] = [b'H' as c_char, b'e' as c_char, b'l' as c_char, b'l' as c_char, b'o' as c_char, 0];
    let empty_str: [c_char; 1] = [0];

    let len1 = unsafe { process_string(test_str.as_ptr()) };
    let len2 = unsafe { process_string(empty_str.as_ptr()) };

    result = result.wrapping_add(len1.wrapping_add(len2));

    unsafe {
        shift_array(block.values.as_mut_ptr(), 4, 1);
    }

    let mut i: c_int = 0;
    while i < block.count {
        result = result.wrapping_add(block.values[i as usize]);
        i += 1;
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix: [[c_int; 4]; 3] = [[0; 4]; 3];
    unsafe {
        init_matrix(matrix.as_mut_ptr());
    }

    result = result.wrapping_add(matrix[0][0].wrapping_add(matrix[2][3]));

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

/// `int arity2(int p1, int p2)`
#[unsafe(no_mangle)]
pub extern "C" fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

/// `int arity3(int p1, int p2, int p3)`
#[unsafe(no_mangle)]
pub extern "C" fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

/// `int arity(unsigned char len, int *params)`
///
/// Note the header advertises `int len`; the definition truncates to a byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_uchar, params: *mut c_int) -> c_int {
    if len < 2 {
        -1
    } else if len == 2 {
        arity2(unsafe { *params }, unsafe { *params.add(1) })
    } else if len == 3 {
        arity3(unsafe { *params }, unsafe { *params.add(1) }, unsafe {
            *params.add(2)
        })
    } else {
        arity4(
            unsafe { *params },
            unsafe { *params.add(1) },
            unsafe { *params.add(2) },
            unsafe { *params.add(3) },
        )
    }
}
