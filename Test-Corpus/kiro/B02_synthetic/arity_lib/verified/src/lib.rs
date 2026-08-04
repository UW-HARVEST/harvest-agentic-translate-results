use std::ffi::{c_char, c_int};

#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shift_array(arr: *mut c_int, size: c_int, positions: c_int) {
    if positions > 0 && positions < size {
        let size = size as usize;
        let positions = positions as usize;
        unsafe {
            std::ptr::copy(arr, arr.add(positions), size - positions);
            for i in 0..positions {
                *arr.add(i) = 0;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_string(str_ptr: *const c_char) -> c_int {
    unsafe {
        if *str_ptr != 0 {
            libc_strlen(str_ptr) as c_int
        } else {
            0
        }
    }
}

unsafe fn libc_strlen(s: *const c_char) -> usize {
    let mut len = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1: c_int = 0b11110000;
    let mask2: c_int = 0b00001111;
    let mask3: c_int = 0b10101010;
    let mask4: c_int = 0b01010101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_matrix(matrix: *mut [[c_int; 4]; 3]) {
    let temp: [[c_int; 4]; 3] = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ];
    unsafe {
        *matrix = temp;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    unsafe {
        let ptr1 = libc_malloc(core::mem::size_of::<c_int>()) as *mut c_int;
        let ptr2 = libc_malloc(core::mem::size_of::<c_int>()) as *mut c_int;

        if ptr1.is_null() || ptr2.is_null() {
            libc_free(ptr1 as *mut u8);
            libc_free(ptr2 as *mut u8);
            return -1;
        }

        *ptr1 = val1;
        *ptr2 = val2;

        let mut result: c_int = if (ptr1 as usize) < (ptr2 as usize) {
            1
        } else if (ptr1 as usize) > (ptr2 as usize) {
            2
        } else {
            3
        };

        let uninit_ptr = ptr1;
        result += if *uninit_ptr > 0 { 10 } else { 0 };

        libc_free(ptr1 as *mut u8);
        libc_free(ptr2 as *mut u8);

        result
    }
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(size: usize) -> *mut u8;
    #[link_name = "free"]
    fn libc_free(ptr: *mut u8);
}

#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    let test_str = b"Hello\0";
    let empty_str = b"\0";

    let len1 = unsafe { process_string(test_str.as_ptr() as *const c_char) };
    let len2 = unsafe { process_string(empty_str.as_ptr() as *const c_char) };

    result += len1 + len2;

    unsafe {
        shift_array(block.values.as_mut_ptr(), 4, 1);
    }

    for i in 0..block.count as usize {
        result += block.values[i];
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0i32; 4]; 3];
    unsafe {
        init_matrix(&mut matrix as *mut [[c_int; 4]; 3]);
    }

    result += matrix[0][0] + matrix[2][3];

    let alloc_result = compare_allocations(param1, param2);
    result += alloc_result;

    if param3 != 0 {
        result = (result.wrapping_mul(param3)) / 100;
    }

    if param4 != 0 {
        result += param4;
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
pub extern "C" fn arity(len: c_int, params: *mut c_int) -> c_int {
    let len = len as u8;
    if len < 2 {
        return -1;
    }
    unsafe {
        if len == 2 {
            arity2(*params, *params.add(1))
        } else if len == 3 {
            arity3(*params, *params.add(1), *params.add(2))
        } else {
            arity4(*params, *params.add(1), *params.add(2), *params.add(3))
        }
    }
}
