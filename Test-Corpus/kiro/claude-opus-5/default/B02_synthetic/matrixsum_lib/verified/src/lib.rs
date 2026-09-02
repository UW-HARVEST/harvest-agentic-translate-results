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

#![allow(non_upper_case_globals)]

use core::ffi::{c_int, c_void};

// The C code allocates the DynamicArray and its backing buffer with
// malloc/realloc and releases them with free. Those allocations are handed out
// across the ABI boundary (init_array returns the raw pointer, free_array takes
// it back), so we must use the very same libc allocator rather than Rust's.
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// int matrix[3][4] = { ... };
//
// Exported as a mutable data object, exactly like the C global, so callers that
// poke at `matrix` directly observe the same layout (row-major, 12 x int).
#[unsafe(no_mangle)]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

/// typedef struct { int *data; size_t size; size_t capacity; } DynamicArray;
#[repr(C)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

/// DynamicArray* init_array(size_t initial_capacity)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    let arr = malloc(core::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
    if arr.is_null() {
        return core::ptr::null_mut();
    }

    // Note: the C code performs `initial_capacity * sizeof(int)` with wrapping
    // (unsigned) semantics; mirror that instead of panicking on overflow.
    let data = malloc(initial_capacity.wrapping_mul(core::mem::size_of::<c_int>())) as *mut c_int;
    (*arr).data = data;
    if data.is_null() {
        free(arr as *mut c_void);
        return core::ptr::null_mut();
    }

    (*arr).size = 0;
    (*arr).capacity = initial_capacity;
    arr
}

/// int expand_array(DynamicArray *arr)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }

    let new_capacity = (*arr).capacity.wrapping_mul(2);
    let new_data = realloc(
        (*arr).data as *mut c_void,
        new_capacity.wrapping_mul(core::mem::size_of::<c_int>()),
    ) as *mut c_int;

    if new_data.is_null() {
        return 0;
    }

    (*arr).data = new_data;
    (*arr).capacity = new_capacity;
    1
}

/// int add_element(DynamicArray *arr, int value)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }

    if (*arr).size >= (*arr).capacity {
        if expand_array(arr) == 0 {
            return 0;
        }
    }

    let idx = (*arr).size;
    (*arr).size = idx.wrapping_add(1);
    *(*arr).data.add(idx) = value;
    1
}

/// void free_array(DynamicArray *arr)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        free((*arr).data as *mut c_void);
        free(arr as *mut c_void);
    }
}

/// int process_flags(int flags)
#[unsafe(no_mangle)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled = (has_read != 0) as c_int;

    let has_write = flags & FLAG_WRITE;
    let write_enabled = (has_write != 0) as c_int;

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = (has_execute != 0) as c_int;

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = (has_delete != 0) as c_int;

    read_enabled
        .wrapping_add(write_enabled)
        .wrapping_add(execute_enabled)
        .wrapping_add(delete_enabled)
}

/// int calculate_matrix_checksum()
#[unsafe(no_mangle)]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;

    for i in 0..3usize {
        for j in 0..4usize {
            // Reads the live (mutable) global, as the C code does.
            sum = sum.wrapping_add(unsafe { matrix[i][j] });
        }
    }

    sum
}

/// int matrixsum(int param1, int param2, int param3, int param4)
#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let valid1 = param1 != 0;
    let valid2 = param2 != 0;
    let valid3 = param3 != 0;
    let valid4 = param4 != 0;

    if valid1 {
        permissions |= FLAG_READ;
    }
    if valid2 {
        permissions |= FLAG_WRITE;
    }
    if valid3 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 {
        permissions |= FLAG_DELETE;
    }

    unsafe {
        let arr = init_array(2);
        if arr.is_null() {
            return -1;
        }

        add_element(arr, param1);
        add_element(arr, param2);
        add_element(arr, param3);
        add_element(arr, param4);

        let mut sum: c_int = 0;
        let mut i: usize = 0;
        while i < (*arr).size {
            sum = sum.wrapping_add(*(*arr).data.add(i));
            i += 1;
        }

        let flag_count = process_flags(permissions);

        let matrix_sum = calculate_matrix_checksum();

        let result = sum
            .wrapping_mul(hex_multiplier)
            .wrapping_add(flag_count.wrapping_mul(hex_base))
            .wrapping_add(matrix_sum & 0xFFF);

        free_array(arr);

        result
    }
}
