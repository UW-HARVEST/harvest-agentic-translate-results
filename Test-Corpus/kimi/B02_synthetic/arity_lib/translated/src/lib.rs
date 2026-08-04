use std::os::raw::{c_int, c_uchar};

#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut u8,
}

fn shift_array(arr: &mut [c_int], positions: usize) {
    let size = arr.len();
    if positions > 0 && positions < size {
        arr.copy_within(0..size - positions, positions);
        for i in 0..positions {
            arr[i] = 0;
        }
    }
}

fn process_string(str: &str) -> c_int {
    if !str.is_empty() {
        str.len() as c_int
    } else {
        0
    }
}

fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
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

fn init_matrix() -> [[c_int; 4]; 3] {
    [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ]
}

fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = Box::new(val1);
    let ptr2 = Box::new(val2);

    let result = if &*ptr1 as *const c_int < &*ptr2 as *const c_int {
        1
    } else if &*ptr1 as *const c_int > &*ptr2 as *const c_int {
        2
    } else {
        3
    };

    let uninit_ptr = &*ptr1;
    let result = result + if *uninit_ptr > 0 { 10 } else { 0 };

    result
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: std::ptr::null_mut(),
    };

    let test_str = "Hello";
    let empty_str = "";

    let len1 = process_string(test_str);
    let len2 = process_string(empty_str);

    result += len1 + len2;

    shift_array(&mut block.values, 1);

    for i in 0..block.count as usize {
        result += block.values[i];
    }

    result = apply_bitmask(result, param1 % 4);

    let matrix = init_matrix();

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
pub extern "C" fn arity(len: c_uchar, params: *const c_int) -> c_int {
    if len < 2 {
        return -1;
    } else if len == 2 {
        unsafe {
            arity2(*params, *params.add(1))
        }
    } else if len == 3 {
        unsafe {
            arity3(*params, *params.add(1), *params.add(2))
        }
    } else {
        unsafe {
            arity4(*params, *params.add(1), *params.add(2), *params.add(3))
        }
    }
}
