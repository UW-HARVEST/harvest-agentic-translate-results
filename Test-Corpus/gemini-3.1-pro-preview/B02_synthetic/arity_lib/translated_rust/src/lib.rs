use std::ffi::{c_char, c_int, CStr};

struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

fn shift_array(arr: &mut [c_int], positions: usize) {
    let size = arr.len();
    if positions > 0 && positions < size {
        arr.copy_within(0..(size - positions), positions);
        for i in 0..positions {
            arr[i] = 0;
        }
    }
}

fn process_string(s: *const c_char) -> c_int {
    unsafe {
        if !s.is_null() && *s != 0 {
            let c_str = CStr::from_ptr(s);
            return c_str.to_bytes().len() as c_int;
        }
    }
    0
}

fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1 = 0b11110000;
    let mask2 = 0b00001111;
    let mask3 = 0b10101010;
    let mask4 = 0b01010101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

fn init_matrix(matrix: &mut [[c_int; 4]; 3]) {
    let temp = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ];
    *matrix = temp;
}

fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = Box::into_raw(Box::new(val1));
    let ptr2 = Box::into_raw(Box::new(val2));

    let mut result = 0;

    if ptr1 < ptr2 {
        result = 1;
    } else if ptr1 > ptr2 {
        result = 2;
    } else {
        result = 3;
    }

    let uninit_ptr = ptr1;
    unsafe {
        result += if *uninit_ptr > 0 { 10 } else { 0 };
    }

    unsafe {
        let _ = Box::from_raw(ptr1);
        let _ = Box::from_raw(ptr2);
    }

    result
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    let test_str = b"Hello\0";
    let empty_str = b"\0";

    let len1 = process_string(test_str.as_ptr() as *const c_char);
    let len2 = process_string(empty_str.as_ptr() as *const c_char);

    result += len1 + len2;

    shift_array(&mut block.values, 1);

    for i in 0..block.count {
        result += block.values[i as usize];
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0; 4]; 3];
    init_matrix(&mut matrix);

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

fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity(len: c_int, params: *mut c_int) -> c_int {
    if len < 2 {
        -1
    } else if len == 2 {
        unsafe { arity2(*params.add(0), *params.add(1)) }
    } else if len == 3 {
        unsafe { arity3(*params.add(0), *params.add(1), *params.add(2)) }
    } else {
        unsafe { arity4(*params.add(0), *params.add(1), *params.add(2), *params.add(3)) }
    }
}
