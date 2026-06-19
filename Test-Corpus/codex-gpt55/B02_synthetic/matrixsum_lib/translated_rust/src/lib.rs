use std::ffi::{c_int, c_void};
use std::mem::size_of;
use std::ptr;

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

#[repr(C)]
pub struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub extern "C" fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    unsafe {
        let arr = malloc(size_of::<DynamicArray>()) as *mut DynamicArray;
        if arr.is_null() {
            return ptr::null_mut();
        }

        let data = malloc(initial_capacity.wrapping_mul(size_of::<c_int>())) as *mut c_int;
        if data.is_null() {
            free(arr as *mut c_void);
            return ptr::null_mut();
        }

        (*arr).data = data;
        (*arr).size = 0;
        (*arr).capacity = initial_capacity;
        arr
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        let new_capacity = (*arr).capacity.wrapping_mul(2);
        let new_data = realloc(
            (*arr).data as *mut c_void,
            new_capacity.wrapping_mul(size_of::<c_int>()),
        ) as *mut c_int;

        if new_data.is_null() {
            return 0;
        }

        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }

    unsafe {
        if (*arr).size >= (*arr).capacity && expand_array(arr) == 0 {
            return 0;
        }

        *(*arr).data.add((*arr).size) = value;
        (*arr).size = (*arr).size.wrapping_add(1);
        1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        unsafe {
            free((*arr).data as *mut c_void);
            free(arr as *mut c_void);
        }
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;

    for i in 0..3 {
        for j in 0..4 {
            sum = sum.wrapping_add(unsafe { matrix[i][j] });
        }
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
    unsafe {
        let mut i: usize = 0;
        while i < (*arr).size {
            sum = sum.wrapping_add(*(*arr).data.add(i));
            i = i.wrapping_add(1);
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
