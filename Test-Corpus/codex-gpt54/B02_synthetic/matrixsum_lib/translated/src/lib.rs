use libc::{c_void, free, malloc, realloc};
use std::ffi::c_int;
use std::ptr;

const MATRIX: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b00000001;
const FLAG_WRITE: c_int = 0b00000010;
const FLAG_EXECUTE: c_int = 0b00000100;
const FLAG_DELETE: c_int = 0b00001000;

struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    unsafe {
        let arr = malloc(std::mem::size_of::<DynamicArray>()) as *mut DynamicArray;
        if arr.is_null() {
            return ptr::null_mut();
        }

        let data = malloc(initial_capacity.wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
        if data.is_null() {
            free(arr.cast::<c_void>());
            return ptr::null_mut();
        }

        (*arr).data = data;
        (*arr).size = 0;
        (*arr).capacity = initial_capacity;
        arr
    }
}

fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        let new_capacity = (*arr).capacity.wrapping_mul(2);
        let new_data = realloc(
            (*arr).data.cast::<c_void>(),
            new_capacity.wrapping_mul(std::mem::size_of::<c_int>()),
        ) as *mut c_int;

        if new_data.is_null() {
            return 0;
        }

        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
        1
    }
}

fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        if (*arr).size >= (*arr).capacity && expand_array(arr) == 0 {
            return 0;
        }

        *(*arr).data.add((*arr).size) = value;
        (*arr).size += 1;
        1
    }
}

fn free_array(arr: *mut DynamicArray) {
    if arr.is_null() {
        return;
    }

    unsafe {
        free((*arr).data.cast::<c_void>());
        free(arr.cast::<c_void>());
    }
}

fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled = (has_read != 0) as c_int;

    let has_write = flags & FLAG_WRITE;
    let write_enabled = (has_write != 0) as c_int;

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = (has_execute != 0) as c_int;

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = (has_delete != 0) as c_int;

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    let mut i = 0;
    while i < 3 {
        let mut j = 0;
        while j < 4 {
            sum = sum.wrapping_add(MATRIX[i][j]);
            j += 1;
        }
        i += 1;
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(
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
    let mut i: usize = 0;
    unsafe {
        while i < (*arr).size {
            sum = sum.wrapping_add(*(*arr).data.add(i));
            i += 1;
        }
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
