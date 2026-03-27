use std::alloc::{alloc, dealloc, Layout};
use std::ptr;

struct DataBlock {
    values: [i32; 4],
    count: i32,
}

#[unsafe(no_mangle)]
pub extern "C" fn shift_array(arr: *mut i32, size: i32, positions: i32) {
    if positions > 0 && positions < size {
        let size = size as usize;
        let positions = positions as usize;
        unsafe {
            ptr::copy(arr, arr.add(positions), size - positions);
            for i in 0..positions {
                *arr.add(i) = 0;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_string(s: *const u8) -> i32 {
    unsafe {
        if *s != 0 {
            let mut len = 0usize;
            while *s.add(len) != 0 {
                len += 1;
            }
            len as i32
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_bitmask(value: i32, operation: i32) -> i32 {
    let mask1: i32 = 0b11110000;
    let mask2: i32 = 0b00001111;
    let mask3: i32 = 0b10101010u32 as i32;
    let mask4: i32 = 0b01010101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn init_matrix(matrix: *mut [i32; 4]) {
    let temp: [[i32; 4]; 3] = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
    unsafe {
        for i in 0..3 {
            for j in 0..4 {
                (*matrix.add(i))[j] = temp[i][j];
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn compare_allocations(val1: i32, val2: i32) -> i32 {
    let layout = Layout::new::<i32>();
    unsafe {
        let ptr1 = alloc(layout) as *mut i32;
        let ptr2 = alloc(layout) as *mut i32;

        if ptr1.is_null() || ptr2.is_null() {
            if !ptr1.is_null() {
                dealloc(ptr1 as *mut u8, layout);
            }
            if !ptr2.is_null() {
                dealloc(ptr2 as *mut u8, layout);
            }
            return -1;
        }

        *ptr1 = val1;
        *ptr2 = val2;

        let mut result: i32;

        if (ptr1 as usize) < (ptr2 as usize) {
            result = 1;
        } else if (ptr1 as usize) > (ptr2 as usize) {
            result = 2;
        } else {
            result = 3;
        }

        let uninit_ptr = ptr1;
        result += if *uninit_ptr > 0 { 10 } else { 0 };

        dealloc(ptr1 as *mut u8, layout);
        dealloc(ptr2 as *mut u8, layout);

        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn arity4(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
    };

    let test_str = b"Hello\0";
    let empty_str = b"\0";

    let len1 = process_string(test_str.as_ptr());
    let len2 = process_string(empty_str.as_ptr());

    result += len1 + len2;

    shift_array(block.values.as_mut_ptr(), 4, 1);

    for i in 0..block.count as usize {
        result += block.values[i];
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0i32; 4]; 3];
    init_matrix(matrix.as_mut_ptr());

    result += matrix[0][0] + matrix[2][3];

    let alloc_result = compare_allocations(param1, param2);
    result += alloc_result;

    if param3 != 0 {
        result = (result * param3) / 100;
    }

    if param4 != 0 {
        result += param4;
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn arity2(p1: i32, p2: i32) -> i32 {
    arity4(p1, p2, 0, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity3(p1: i32, p2: i32, p3: i32) -> i32 {
    arity4(p1, p2, p3, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity(len: u8, params: *const i32) -> i32 {
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
