use std::os::raw::c_int;

struct DataBlock {
    values: [c_int; 4],
    count: c_int,
}

fn shift_array(arr: &mut [c_int], size: c_int, positions: c_int) {
    let size = size as usize;
    let positions = positions as usize;
    if positions > 0 && positions < size {
        // memmove right, then zero-fill left
        let mut i = size - 1;
        while i >= positions {
            arr[i] = arr[i - positions];
            if i == positions {
                break;
            }
            i -= 1;
        }
        for i in 0..positions {
            arr[i] = 0;
        }
    }
}

fn process_string(s: &[u8]) -> c_int {
    if !s.is_empty() && s[0] != 0 {
        // strlen: count until null terminator
        let mut len = 0;
        while len < s.len() && s[len] != 0 {
            len += 1;
        }
        len as c_int
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

fn init_matrix(matrix: &mut [[c_int; 4]; 3]) {
    let temp: [[c_int; 4]; 3] = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ];
    for i in 0..3 {
        for j in 0..4 {
            matrix[i][j] = temp[i][j];
        }
    }
}

fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = Box::new(val1);
    let ptr2 = Box::new(val2);

    let addr1 = &*ptr1 as *const c_int as usize;
    let addr2 = &*ptr2 as *const c_int as usize;

    let mut result: c_int;

    if addr1 < addr2 {
        result = 1;
    } else if addr1 > addr2 {
        result = 2;
    } else {
        result = 3;
    }

    // uninit_ptr = ptr1; result += (*uninit_ptr > 0) ? 10 : 0;
    result += if *ptr1 > 0 { 10 } else { 0 };

    result
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
    };

    let test_str = b"Hello\0";
    let empty_str = b"\0";

    let len1 = process_string(test_str);
    let len2 = process_string(empty_str);

    result += len1 + len2;

    shift_array(&mut block.values, 4, 1);

    for i in 0..block.count as usize {
        result += block.values[i];
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0i32; 4]; 3];
    init_matrix(&mut matrix);

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

fn arity2(p1: c_int, p2: c_int) -> c_int {
    arity4(p1, p2, 0, 0)
}

fn arity3(p1: c_int, p2: c_int, p3: c_int) -> c_int {
    arity4(p1, p2, p3, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn arity(len: u8, params: *mut c_int) -> c_int {
    if len < 2 {
        return -1;
    }
    let params_slice = unsafe { std::slice::from_raw_parts(params, len as usize) };
    if len == 2 {
        arity2(params_slice[0], params_slice[1])
    } else if len == 3 {
        arity3(params_slice[0], params_slice[1], params_slice[2])
    } else {
        arity4(params_slice[0], params_slice[1], params_slice[2], params_slice[3])
    }
}
