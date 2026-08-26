use std::os::raw::c_int;
use std::os::raw::c_ulong;

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
    size: c_ulong,
    capacity: c_ulong,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_array(initial_capacity: c_ulong) -> *mut DynamicArray {
    let layout_arr = std::alloc::Layout::new::<DynamicArray>();
    let arr = unsafe { std::alloc::alloc(layout_arr) as *mut DynamicArray };
    if arr.is_null() {
        return std::ptr::null_mut();
    }
    let data_layout = std::alloc::Layout::array::<c_int>(initial_capacity as usize);
    let data = match data_layout {
        Ok(l) => unsafe { std::alloc::alloc(l) as *mut c_int },
        Err(_) => {
            unsafe { std::alloc::dealloc(arr as *mut u8, layout_arr) };
            return std::ptr::null_mut();
        }
    };
    if data.is_null() {
        unsafe { std::alloc::dealloc(arr as *mut u8, layout_arr) };
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
pub unsafe extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }
    unsafe {
        let new_capacity = (*arr).capacity * 2;
        let old_layout = std::alloc::Layout::array::<c_int>((*arr).capacity as usize).unwrap();
        let new_data = std::alloc::realloc(
            (*arr).data as *mut u8,
            old_layout,
            new_capacity as usize * std::mem::size_of::<c_int>(),
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
pub unsafe extern "C" fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }
    unsafe {
        if (*arr).size >= (*arr).capacity {
            if expand_array(arr) == 0 {
                return 0;
            }
        }
        *(*arr).data.add((*arr).size as usize) = value;
        (*arr).size += 1;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        unsafe {
            let data_layout =
                std::alloc::Layout::array::<c_int>((*arr).capacity as usize).unwrap();
            std::alloc::dealloc((*arr).data as *mut u8, data_layout);
            std::alloc::dealloc(arr as *mut u8, std::alloc::Layout::new::<DynamicArray>());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let read_enabled = c_int::from(flags & FLAG_READ != 0);
    let write_enabled = c_int::from(flags & FLAG_WRITE != 0);
    let execute_enabled = c_int::from(flags & FLAG_EXECUTE != 0);
    let delete_enabled = c_int::from(flags & FLAG_DELETE != 0);
    read_enabled + write_enabled + execute_enabled + delete_enabled
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    unsafe {
        for row in &matrix {
            for &val in row {
                sum += val;
            }
        }
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    if param1 != 0 { permissions |= FLAG_READ; }
    if param2 != 0 { permissions |= FLAG_WRITE; }
    if param3 != 0 { permissions |= FLAG_EXECUTE; }
    if param4 != 0 { permissions |= FLAG_DELETE; }

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
    unsafe {
        for i in 0..(*arr).size as usize {
            sum = sum.wrapping_add(*(*arr).data.add(i));
        }
    }

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();

    let result = (sum.wrapping_mul(hex_multiplier))
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF);

    unsafe { free_array(arr) };

    result
}
