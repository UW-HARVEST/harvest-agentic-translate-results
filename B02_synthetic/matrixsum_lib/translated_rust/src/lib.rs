use std::os::raw::c_int;
use std::ptr;

#[unsafe(no_mangle)]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b00000001;
const FLAG_WRITE: c_int = 0b00000010;
const FLAG_EXECUTE: c_int = 0b00000100;
const FLAG_DELETE: c_int = 0b00001000;

#[repr(C)]
pub struct DynamicArray {
    data: *mut c_int,
    size: usize,
    capacity: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    let arr = Box::new(DynamicArray {
        data: ptr::null_mut(),
        size: 0,
        capacity: initial_capacity,
    });
    let arr = Box::into_raw(arr);
    unsafe {
        let data = libc_malloc(initial_capacity * std::mem::size_of::<c_int>()) as *mut c_int;
        if data.is_null() {
            let _ = Box::from_raw(arr);
            return ptr::null_mut();
        }
        (*arr).data = data;
    }
    arr
}

#[unsafe(no_mangle)]
pub extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }
    unsafe {
        let new_capacity = (*arr).capacity * 2;
        let new_data = libc_realloc(
            (*arr).data as *mut u8,
            new_capacity * std::mem::size_of::<c_int>(),
        ) as *mut c_int;
        if new_data.is_null() {
            return 0;
        }
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }
    unsafe {
        if (*arr).size >= (*arr).capacity {
            if expand_array(arr) == 0 {
                return 0;
            }
        }
        *(*arr).data.add((*arr).size) = value;
        (*arr).size += 1;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        unsafe {
            if !(*arr).data.is_null() {
                libc_free((*arr).data as *mut u8);
            }
            let _ = Box::from_raw(arr);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let read_enabled = ((flags & FLAG_READ) != 0) as c_int;
    let write_enabled = ((flags & FLAG_WRITE) != 0) as c_int;
    let execute_enabled = ((flags & FLAG_EXECUTE) != 0) as c_int;
    let delete_enabled = ((flags & FLAG_DELETE) != 0) as c_int;
    read_enabled + write_enabled + execute_enabled + delete_enabled
}

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0;

    if param1 != 0 { permissions |= FLAG_READ; }
    if param2 != 0 { permissions |= FLAG_WRITE; }
    if param3 != 0 { permissions |= FLAG_EXECUTE; }
    if param4 != 0 { permissions |= FLAG_DELETE; }

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
        for i in 0..(*arr).size {
            sum = sum.wrapping_add(*(*arr).data.add(i));
        }
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

// Minimal malloc/realloc/free wrappers using libc
extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

unsafe fn libc_malloc(size: usize) -> *mut u8 {
    unsafe { malloc(size) }
}

unsafe fn libc_realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    unsafe { realloc(ptr, size) }
}

unsafe fn libc_free(ptr: *mut u8) {
    unsafe { free(ptr) }
}
