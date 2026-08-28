use std::ffi::{c_char, c_int, c_void};
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

        // SAFETY: The C API requires arr to reference at least size integers.
        unsafe {
            ptr::copy(arr, arr.add(positions), count);
            ptr::write_bytes(arr, 0, positions);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(string: *const c_char) -> c_int {
    // SAFETY: As in C, callers must provide a valid NUL-terminated string.
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
    const INITIAL: [[c_int; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];

    // SAFETY: The C API requires matrix to reference three rows of four integers.
    unsafe {
        ptr::copy_nonoverlapping(INITIAL.as_ptr(), matrix, INITIAL.len());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    // SAFETY: malloc/free are called with their standard C contracts.
    unsafe {
        let ptr1 = malloc(size_of::<c_int>()).cast::<c_int>();
        let ptr2 = malloc(size_of::<c_int>()).cast::<c_int>();

        if ptr1.is_null() || ptr2.is_null() {
            free(ptr1.cast());
            free(ptr2.cast());
            return -1;
        }

        ptr::write(ptr1, val1);
        ptr::write(ptr2, val2);

        let mut result = if (ptr1 as usize) < (ptr2 as usize) {
            1
        } else if (ptr1 as usize) > (ptr2 as usize) {
            2
        } else {
            3
        };

        if *ptr1 > 0 {
            result += 10;
        }

        free(ptr1.cast());
        free(ptr2.cast());
        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut values = [param1, param2, param3, param4];
    let test_string = b"Hello\0";
    let empty_string = b"\0";

    // SAFETY: Both byte arrays are valid NUL-terminated C strings.
    let len1 = unsafe { process_string(test_string.as_ptr().cast()) };
    let len2 = unsafe { process_string(empty_string.as_ptr().cast()) };
    let mut result = len1.wrapping_add(len2);

    // SAFETY: values contains four contiguous integers.
    unsafe {
        shift_array(values.as_mut_ptr(), 4, 1);
    }
    for value in values {
        result = result.wrapping_add(value);
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0; 4]; 3];
    // SAFETY: matrix has exactly the layout required by the C parameter.
    unsafe {
        init_matrix(matrix.as_mut_ptr());
    }
    result = result.wrapping_add(matrix[0][0]).wrapping_add(matrix[2][3]);

    result = result.wrapping_add(compare_allocations(param1, param2));

    if param3 != 0 {
        result = result.wrapping_mul(param3) / 100;
    }
    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn arity2(param1: c_int, param2: c_int) -> c_int {
    arity4(param1, param2, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity3(param1: c_int, param2: c_int, param3: c_int) -> c_int {
    arity4(param1, param2, param3, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn arity(len: c_int, params: *mut c_int) -> c_int {
    // The implementation defines len as unsigned char despite the public header's int.
    match len as u8 {
        0..=1 => -1,
        2 => unsafe { arity2(*params, *params.add(1)) },
        3 => unsafe { arity3(*params, *params.add(1), *params.add(2)) },
        _ => unsafe { arity4(*params, *params.add(1), *params.add(2), *params.add(3)) },
    }
}
