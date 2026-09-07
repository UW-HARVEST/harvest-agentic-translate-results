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

//! Faithful Rust translation of `c_src/src/lib.c`.
//!
//! Every public symbol exported by the C shared library is reproduced here with
//! the identical linker name, C ABI signature and observable behaviour
//! (including the quirks of the original code, which are intentionally *not*
//! fixed).

#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C code performs its heap traffic with the platform allocator
// (`malloc`/`free`) and its string/memory work with `strlen`/`memmove`.
// `compare_allocations()` observably compares the *addresses* returned by two
// consecutive `malloc(sizeof(int))` calls, so the platform allocator has to be
// used here as well (rather than Rust's `alloc`) for the results to match the C
// library call-for-call.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
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
    label: *mut c_char,
}

/// ```c
/// void shift_array(int *arr, int size, int positions);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    if positions > 0 && positions < size {
        // memmove(arr + positions, arr, (size - positions) * sizeof(int));
        let elems = size.wrapping_sub(positions) as isize;
        unsafe {
            memmove(
                arr.offset(positions as isize) as *mut c_void,
                arr as *const c_void,
                (elems as usize).wrapping_mul(core::mem::size_of::<c_int>()),
            );
        }
        let mut i: c_int = 0;
        while i < positions {
            unsafe {
                *arr.offset(i as isize) = 0;
            }
            i += 1;
        }
    }
}

/// ```c
/// int process_string(const char *str);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str: *const c_char) -> c_int {
    // Dereferences `str` unconditionally, exactly like the C original.
    if unsafe { *str } != 0 {
        return unsafe { strlen(str) } as c_int;
    }
    0
}

/// ```c
/// int apply_bitmask(int value, int operation);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
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

/// ```c
/// void init_matrix(int matrix[3][4]);
/// ```
///
/// An `int (*)[4]` parameter is a plain pointer at the ABI level.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut c_int) {
    let temp: [[c_int; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];

    for i in 0..3usize {
        for j in 0..4usize {
            unsafe {
                *matrix.add(i * 4 + j) = temp[i][j];
            }
        }
    }
}

/// ```c
/// int compare_allocations(int val1, int val2);
/// ```
#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = unsafe { malloc(core::mem::size_of::<c_int>()) } as *mut c_int;
    let ptr2 = unsafe { malloc(core::mem::size_of::<c_int>()) } as *mut c_int;

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

    let mut result: c_int = 0;

    // The C code compares the two unrelated pointers; gcc emits an unsigned
    // address comparison, which is what `usize` ordering does here.
    if (ptr1 as usize) < (ptr2 as usize) {
        result = 1;
    } else if (ptr1 as usize) > (ptr2 as usize) {
        result = 2;
    } else {
        result = 3;
    }

    uninit_ptr = ptr1;
    result = result.wrapping_add(if unsafe { *uninit_ptr } > 0 { 10 } else { 0 });

    unsafe {
        free(ptr1 as *mut c_void);
        free(ptr2 as *mut c_void);
    }

    result
}

/// ```c
/// int arity4(int param1, int param2, int param3, int param4);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: core::ptr::null_mut(),
    };

    let test_str: [c_char; 6] = [
        'H' as c_char,
        'e' as c_char,
        'l' as c_char,
        'l' as c_char,
        'o' as c_char,
        0,
    ];
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

    result = apply_bitmask(result, param1.wrapping_rem(4));

    let mut matrix: [[c_int; 4]; 3] = [[0; 4]; 3];
    unsafe {
        init_matrix(matrix.as_mut_ptr() as *mut c_int);
    }

    result = result
        .wrapping_add(matrix[0][0])
        .wrapping_add(matrix[2][3]);

    let alloc_result = compare_allocations(param1, param2);
    result = result.wrapping_add(alloc_result);

    if param3 != 0 {
        result = result.wrapping_mul(param3).wrapping_div(100);
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    result
}

/// ```c
/// int arity2(int p1, int p2);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

/// ```c
/// int arity3(int p1, int p2, int p3);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

/// ```c
/// int arity(unsigned char len, int *params);   // src/lib.c definition
/// int arity(int len, int *params);             // include/lib.h declaration
/// ```
///
/// The public header declares `len` as `int` while the definition takes an
/// `unsigned char`; gcc therefore only ever looks at the low 8 bits of the
/// incoming argument register (`mov %edi,%eax; mov %al,...`) and compares them
/// as an unsigned byte. The parameter is accepted as `c_int` here and truncated
/// the same way so that callers using either prototype observe identical
/// behaviour (e.g. `len == 256` truncates to 0 and yields -1).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_int, params: *const c_int) -> c_int {
    let len: u8 = (len as u32 & 0xff) as u8;

    if len < 2 {
        -1
    } else if len == 2 {
        arity2(unsafe { *params.offset(0) }, unsafe { *params.offset(1) })
    } else if len == 3 {
        arity3(
            unsafe { *params.offset(0) },
            unsafe { *params.offset(1) },
            unsafe { *params.offset(2) },
        )
    } else {
        arity4(
            unsafe { *params.offset(0) },
            unsafe { *params.offset(1) },
            unsafe { *params.offset(2) },
            unsafe { *params.offset(3) },
        )
    }
}
