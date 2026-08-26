use std::ffi::{c_int, c_void};

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

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
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
    let arr = unsafe { malloc(size_of::<DynamicArray>()) }.cast::<DynamicArray>();
    if arr.is_null() {
        return std::ptr::null_mut();
    }

    let data = unsafe { malloc(initial_capacity.wrapping_mul(size_of::<c_int>())) }.cast::<c_int>();
    if data.is_null() {
        unsafe { free(arr.cast::<c_void>()) };
        return std::ptr::null_mut();
    }

    unsafe {
        (*arr).data = data;
        (*arr).size = 0;
        (*arr).capacity = initial_capacity;
    }
    arr
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }

    let new_capacity = unsafe { (*arr).capacity }.wrapping_mul(2);
    let new_data = unsafe {
        realloc(
            (*arr).data.cast::<c_void>(),
            new_capacity.wrapping_mul(size_of::<c_int>()),
        )
    }
    .cast::<c_int>();

    if new_data.is_null() {
        return 0;
    }

    unsafe {
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    1
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }

    if unsafe { (*arr).size >= (*arr).capacity } && unsafe { expand_array(arr) } == 0 {
        return 0;
    }

    unsafe {
        let index = (*arr).size;
        *(*arr).data.add(index) = value;
        (*arr).size = index.wrapping_add(1);
    }
    1
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        unsafe {
            free((*arr).data.cast::<c_void>());
            free(arr.cast::<c_void>());
        }
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled = c_int::from(has_read != 0);

    let has_write = flags & FLAG_WRITE;
    let write_enabled = c_int::from(has_write != 0);

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = c_int::from(has_execute != 0);

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = c_int::from(has_delete != 0);

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

#[unsafe(no_mangle)]
#[inline(never)]
pub unsafe extern "C" fn calculate_matrix_checksum() -> c_int {
    let matrix_ptr = (&raw const matrix).cast::<c_int>();
    let mut sum: c_int = 0;

    for index in 0..12 {
        sum = sum.wrapping_add(unsafe { *matrix_ptr.add(index) });
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
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;
    let mut permissions: c_int = 0b0000;

    let check1 = param1;
    let valid1 = check1 != 0;

    let check2 = param2;
    let valid2 = check2 != 0;

    let check3 = param3;
    let valid3 = check3 != 0;

    let check4 = param4;
    let valid4 = check4 != 0;

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

    let arr = unsafe { init_array(2) };
    if arr.is_null() {
        return -1;
    }

    unsafe {
        add_element(arr, param1);
        add_element(arr, param2);
        add_element(arr, param3);
        add_element(arr, param4);
    }

    let mut sum: c_int = 0;
    let mut index = 0;
    while index < unsafe { (*arr).size } {
        sum = sum.wrapping_add(unsafe { *(*arr).data.add(index) });
        index += 1;
    }

    let flag_count = process_flags(permissions);
    let matrix_sum = unsafe { calculate_matrix_checksum() };
    let result = sum
        .wrapping_mul(hex_multiplier)
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF);

    unsafe { free_array(arr) };

    result
}
