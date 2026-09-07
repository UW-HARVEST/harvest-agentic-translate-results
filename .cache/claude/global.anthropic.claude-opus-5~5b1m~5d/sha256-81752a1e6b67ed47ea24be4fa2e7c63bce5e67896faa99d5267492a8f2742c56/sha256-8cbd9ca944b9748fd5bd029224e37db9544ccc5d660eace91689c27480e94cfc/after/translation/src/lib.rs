// Rust translation of c_src/src/lib.c
//
// Original copyright notice from the C source is reproduced below.
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
#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_void};

/// `size_t` equivalent for the target ABI.
#[allow(non_camel_case_types)]
type size_t = usize;

// The C code manages its `DynamicArray` allocations with malloc/realloc/free.
// Use the very same C allocator so that ownership can cross the FFI boundary
// exactly as it does in the original library.
extern "C" {
    fn malloc(size: size_t) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// ---------------------------------------------------------------------------
// Global data
// ---------------------------------------------------------------------------

// int matrix[3][4] = {
//     {0x01, 0x02, 0x03, 0x04},
//     {0x10, 0x20, 0x30, 0x40},
//     {0xA1, 0xB2, 0xC3, 0xD4}
// };
//
// This is a mutable, externally visible data object in the C library, so it is
// exported here as a `static mut` with the identical name and layout.
#[unsafe(no_mangle)]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

// ---------------------------------------------------------------------------
// typedef struct { int *data; size_t size; size_t capacity; } DynamicArray;
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: size_t,
    pub capacity: size_t,
}

/// Size in bytes of one `int`, as `sizeof(int)` in the C source.
const SIZEOF_INT: size_t = core::mem::size_of::<c_int>();

// ---------------------------------------------------------------------------
// DynamicArray* init_array(size_t initial_capacity)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_array(initial_capacity: size_t) -> *mut DynamicArray {
    let arr = malloc(core::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
    if arr.is_null() {
        return core::ptr::null_mut();
    }

    // `initial_capacity * sizeof(int)` is an unsigned (size_t) multiplication in
    // C, so it wraps on overflow rather than trapping.
    let bytes = initial_capacity.wrapping_mul(SIZEOF_INT);
    let data = malloc(bytes) as *mut c_int;
    (*arr).data = data;
    if (*arr).data.is_null() {
        free(arr as *mut c_void);
        return core::ptr::null_mut();
    }

    (*arr).size = 0;
    (*arr).capacity = initial_capacity;
    arr
}

// ---------------------------------------------------------------------------
// int expand_array(DynamicArray *arr)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }

    let new_capacity = (*arr).capacity.wrapping_mul(2);
    let new_data =
        realloc((*arr).data as *mut c_void, new_capacity.wrapping_mul(SIZEOF_INT)) as *mut c_int;

    if new_data.is_null() {
        return 0;
    }

    (*arr).data = new_data;
    (*arr).capacity = new_capacity;
    1
}

// ---------------------------------------------------------------------------
// int add_element(DynamicArray *arr, int value)
// ---------------------------------------------------------------------------
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
    *(*arr).data.add(idx) = value;
    (*arr).size = idx.wrapping_add(1);
    1
}

// ---------------------------------------------------------------------------
// void free_array(DynamicArray *arr)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        free((*arr).data as *mut c_void);
        free(arr as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// int process_flags(int flags)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let count: c_int;

    let has_read = flags & FLAG_READ;
    let read_enabled = (has_read != 0) as c_int;

    let has_write = flags & FLAG_WRITE;
    let write_enabled = (has_write != 0) as c_int;

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = (has_execute != 0) as c_int;

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = (has_delete != 0) as c_int;

    count = read_enabled
        .wrapping_add(write_enabled)
        .wrapping_add(execute_enabled)
        .wrapping_add(delete_enabled);

    count
}

// ---------------------------------------------------------------------------
// int calculate_matrix_checksum()
//
// Note: declared with an empty (unprototyped) parameter list in C; the ABI for
// a call with no arguments is identical to `extern "C" fn() -> c_int`.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;

    let m = &raw const matrix;
    let mut i: c_int = 0;
    while i < 3 {
        let mut j: c_int = 0;
        while j < 4 {
            // Read through a raw pointer: `matrix` is publicly mutable data.
            let v = unsafe { (*m)[i as usize][j as usize] };
            sum = sum.wrapping_add(v);
            j += 1;
        }
        i += 1;
    }

    sum
}

// ---------------------------------------------------------------------------
// int matrixsum(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrixsum(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let result: c_int;

    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let check1 = param1;
    let valid1 = (check1 != 0) as c_int;

    let check2 = param2;
    let valid2 = (check2 != 0) as c_int;

    let check3 = param3;
    let valid3 = (check3 != 0) as c_int;

    let check4 = param4;
    let valid4 = (check4 != 0) as c_int;

    if valid1 != 0 {
        permissions |= FLAG_READ;
    }
    if valid2 != 0 {
        permissions |= FLAG_WRITE;
    }
    if valid3 != 0 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 != 0 {
        permissions |= FLAG_DELETE;
    }

    let arr = init_array(2);
    if arr.is_null() {
        return -1;
    }

    add_element(arr, param1);
    add_element(arr, param2);
    add_element(arr, param3);
    add_element(arr, param4);

    let mut sum: c_int = 0;
    let mut i: size_t = 0;
    while i < (*arr).size {
        sum = sum.wrapping_add(*(*arr).data.add(i));
        i += 1;
    }

    let flag_count = process_flags(permissions);

    let matrix_sum = calculate_matrix_checksum();

    result = sum
        .wrapping_mul(hex_multiplier)
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF);

    free_array(arr);

    result
}
