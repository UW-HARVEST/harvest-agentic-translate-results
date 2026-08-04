// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of c_src/src/lib.c — preserves byte-identical behavior.

use std::ffi::c_int;
use std::os::raw::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// Globally exported `matrix` variable matching the C definition:
// int matrix[3][4] = {...};
#[no_mangle]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

#[repr(C)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

#[no_mangle]
pub unsafe extern "C" fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    let arr = malloc(std::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
    if arr.is_null() {
        return std::ptr::null_mut();
    }

    let data = malloc(initial_capacity.wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
    if data.is_null() {
        free(arr as *mut c_void);
        return std::ptr::null_mut();
    }

    (*arr).data = data;
    (*arr).size = 0;
    (*arr).capacity = initial_capacity;
    arr
}

#[no_mangle]
pub unsafe extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }

    let new_capacity = (*arr).capacity.wrapping_mul(2);
    let new_data = realloc(
        (*arr).data as *mut c_void,
        new_capacity.wrapping_mul(std::mem::size_of::<c_int>()),
    ) as *mut c_int;

    if new_data.is_null() {
        return 0;
    }

    (*arr).data = new_data;
    (*arr).capacity = new_capacity;
    1
}

#[no_mangle]
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

#[no_mangle]
pub unsafe extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        free((*arr).data as *mut c_void);
        free(arr as *mut c_void);
    }
}

#[no_mangle]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled: c_int = if has_read != 0 { 1 } else { 0 };

    let has_write = flags & FLAG_WRITE;
    let write_enabled: c_int = if has_write != 0 { 1 } else { 0 };

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled: c_int = if has_execute != 0 { 1 } else { 0 };

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled: c_int = if has_delete != 0 { 1 } else { 0 };

    read_enabled
        .wrapping_add(write_enabled)
        .wrapping_add(execute_enabled)
        .wrapping_add(delete_enabled)
}

#[no_mangle]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    unsafe {
        for i in 0..3 {
            for j in 0..4 {
                sum = sum.wrapping_add(matrix[i][j]);
            }
        }
    }
    sum
}

#[no_mangle]
pub extern "C" fn matrixsum(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let valid1: c_int = if param1 != 0 { 1 } else { 0 };
    let valid2: c_int = if param2 != 0 { 1 } else { 0 };
    let valid3: c_int = if param3 != 0 { 1 } else { 0 };
    let valid4: c_int = if param4 != 0 { 1 } else { 0 };

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
        let size = (*arr).size;
        for i in 0..size {
            sum = sum.wrapping_add(*(*arr).data.add(i));
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
