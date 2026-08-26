use std::ffi::c_char;
use std::os::raw::c_int;
use std::ptr;
use std::slice;

#[repr(C)]
struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut c_char,
}

fn shift_array(arr: &mut [c_int], positions: usize) {
    let size = arr.len();
    if positions > 0 && positions < size {
        arr.copy_within(0..(size - positions), positions);
        for item in &mut arr[..positions] {
            *item = 0;
        }
    }
}

fn process_string_bytes(bytes: &[u8]) -> c_int {
    if !bytes.is_empty() && bytes[0] != 0 {
        bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len()) as c_int
    } else {
        0
    }
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
    let temp = [[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12]];
    *matrix = temp;
}

fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    let ptr1 = Box::into_raw(Box::new(val1));
    let ptr2 = Box::into_raw(Box::new(val2));

    let mut result = if (ptr1 as usize) < (ptr2 as usize) {
        1
    } else if (ptr1 as usize) > (ptr2 as usize) {
        2
    } else {
        3
    };

    result += if val1 > 0 { 10 } else { 0 };

    unsafe {
        drop(Box::from_raw(ptr1));
        drop(Box::from_raw(ptr2));
    }

    result
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: ptr::null_mut(),
    };

    let test_str = b"Hello\0";
    let empty_str = b"\0";

    let len1 = process_string_bytes(test_str);
    let len2 = process_string_bytes(empty_str);

    result += len1 + len2;

    shift_array(&mut block.values, 1);

    for i in 0..block.count as usize {
        result += block.values[i];
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
        return -1;
    }
    if params.is_null() {
        return -1;
    }

    let needed = if len == 2 { 2 } else if len == 3 { 3 } else { 4 };
    let params = unsafe { slice::from_raw_parts(params as *const c_int, needed) };

    if len == 2 {
        arity2(params[0], params[1])
    } else if len == 3 {
        arity3(params[0], params[1], params[2])
    } else {
        arity4(params[0], params[1], params[2], params[3])
    }
}
