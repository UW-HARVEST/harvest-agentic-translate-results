use std::ffi::{c_int, c_void};
use std::ptr;

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

#[repr(C)]
pub struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

#[unsafe(no_mangle)]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    let array = unsafe { malloc(size_of::<DynamicArray>()) }.cast::<DynamicArray>();
    if array.is_null() {
        return ptr::null_mut();
    }

    let data_size = initial_capacity.wrapping_mul(size_of::<c_int>());
    let data = unsafe { malloc(data_size) }.cast::<c_int>();
    if data.is_null() {
        unsafe { free(array.cast()) };
        return ptr::null_mut();
    }

    unsafe {
        (*array).data = data;
        (*array).size = 0;
        (*array).capacity = initial_capacity;
    }
    array
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn expand_array(array: *mut DynamicArray) -> c_int {
    if array.is_null() {
        return 0;
    }

    let new_capacity = unsafe { (*array).capacity }.wrapping_mul(2);
    let new_size = new_capacity.wrapping_mul(size_of::<c_int>());
    let new_data = unsafe { realloc((*array).data.cast(), new_size) }.cast::<c_int>();
    if new_data.is_null() {
        return 0;
    }

    unsafe {
        (*array).data = new_data;
        (*array).capacity = new_capacity;
    }
    1
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn add_element(array: *mut DynamicArray, value: c_int) -> c_int {
    if array.is_null() {
        return 0;
    }

    if unsafe { (*array).size >= (*array).capacity } && unsafe { expand_array(array) } == 0 {
        return 0;
    }

    unsafe {
        let index = (*array).size;
        *(*array).data.add(index) = value;
        (*array).size = index.wrapping_add(1);
    }
    1
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn free_array(array: *mut DynamicArray) {
    if !array.is_null() {
        unsafe {
            free((*array).data.cast());
            free(array.cast());
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let read_enabled = c_int::from(flags & FLAG_READ != 0);
    let write_enabled = c_int::from(flags & FLAG_WRITE != 0);
    let execute_enabled = c_int::from(flags & FLAG_EXECUTE != 0);
    let delete_enabled = c_int::from(flags & FLAG_DELETE != 0);

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    let base = ptr::addr_of!(matrix).cast::<c_int>();

    for index in 0..12 {
        sum = sum.wrapping_add(unsafe { *base.add(index) });
    }
    sum
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn matrixsum(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut permissions = 0;
    if param1 != 0 {
        permissions |= FLAG_READ;
    }
    if param2 != 0 {
        permissions |= FLAG_WRITE;
    }
    if param3 != 0 {
        permissions |= FLAG_EXECUTE;
    }
    if param4 != 0 {
        permissions |= FLAG_DELETE;
    }

    let array = unsafe { init_array(2) };
    if array.is_null() {
        return -1;
    }

    unsafe {
        add_element(array, param1);
        add_element(array, param2);
        add_element(array, param3);
        add_element(array, param4);
    }

    let mut sum: c_int = 0;
    let mut index = 0;
    while index < unsafe { (*array).size } {
        sum = sum.wrapping_add(unsafe { *(*array).data.add(index) });
        index += 1;
    }

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();
    let result = sum
        .wrapping_mul(0x10)
        .wrapping_add(flag_count.wrapping_mul(0xFF))
        .wrapping_add(matrix_sum & 0xFFF);

    unsafe { free_array(array) };
    result
}
