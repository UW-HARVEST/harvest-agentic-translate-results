use std::ffi::{c_char, c_int, c_uchar, c_void};
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    if positions > 0 && positions < size {
        let positions = positions as usize;
        let count = (size as usize) - positions;

        unsafe {
            ptr::copy(arr, arr.add(positions), count);
            ptr::write_bytes(arr, 0, positions);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(string: *const c_char) -> c_int {
    if unsafe { *string } != 0 {
        unsafe { strlen(string) as c_int }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    match operation {
        0 => value & 0b1111_0000,
        1 => value & 0b0000_1111,
        2 => value | 0b1010_1010,
        3 => value ^ 0b0101_0101,
        _ => value,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut [c_int; 4]) {
    const VALUES: [[c_int; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];

    for (row_index, row) in VALUES.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            unsafe {
                (*matrix.add(row_index))[column_index] = *value;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = unsafe { malloc(size_of::<c_int>()) }.cast::<c_int>();
    let ptr2 = unsafe { malloc(size_of::<c_int>()) }.cast::<c_int>();

    if ptr1.is_null() || ptr2.is_null() {
        unsafe {
            free(ptr1.cast());
            free(ptr2.cast());
        }
        return -1;
    }

    unsafe {
        *ptr1 = val1;
        *ptr2 = val2;
    }

    let mut result = if ptr1.addr() < ptr2.addr() {
        1
    } else if ptr1.addr() > ptr2.addr() {
        2
    } else {
        3
    };

    if unsafe { *ptr1 } > 0 {
        result += 10;
    }

    unsafe {
        free(ptr1.cast());
        free(ptr2.cast());
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut values = [param1, param2, param3, param4];
    let test_string = b"Hello\0";
    let empty_string = b"\0";

    let len1 = unsafe { process_string(test_string.as_ptr().cast()) };
    let len2 = unsafe { process_string(empty_string.as_ptr().cast()) };
    let mut result = len1.wrapping_add(len2);

    unsafe {
        shift_array(values.as_mut_ptr(), 4, 1);
    }

    for value in values {
        result = result.wrapping_add(value);
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0; 4]; 3];
    unsafe {
        init_matrix(matrix.as_mut_ptr());
    }

    result = result.wrapping_add(matrix[0][0]).wrapping_add(matrix[2][3]);

    let alloc_result = unsafe { compare_allocations(param1, param2) };
    result = result.wrapping_add(alloc_result);

    if param3 != 0 {
        result = result.wrapping_mul(param3) / 100;
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_uchar, params: *mut c_int) -> c_int {
    if len < 2 {
        -1
    } else if len == 2 {
        unsafe { arity2(*params, *params.add(1)) }
    } else if len == 3 {
        unsafe { arity3(*params, *params.add(1), *params.add(2)) }
    } else {
        unsafe { arity4(*params, *params.add(1), *params.add(2), *params.add(3)) }
    }
}
