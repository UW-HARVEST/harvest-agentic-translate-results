use libc::{c_int, c_uchar, free, malloc};
use std::mem;
use std::ptr;

struct DataBlock {
    values: [c_int; 4],
    count: c_int,
    label: *mut u8,
}

fn shift_array(arr: &mut [c_int], positions: c_int) {
    let size = arr.len() as c_int;
    if positions > 0 && positions < size {
        let positions_usize = positions as usize;
        let count = (size - positions) as usize;
        arr.copy_within(0..count, positions_usize);
        for item in &mut arr[..positions_usize] {
            *item = 0;
        }
    }
}

fn process_string(bytes: &[u8]) -> c_int {
    if bytes.first().copied().unwrap_or(0) != 0 {
        bytes.iter().take_while(|&&byte| byte != 0).count() as c_int
    } else {
        0
    }
}

fn apply_bitmask(value: c_int, operation: c_int) -> c_int {
    let mask1 = 0b1111_0000;
    let mask2 = 0b0000_1111;
    let mask3 = 0b1010_1010;
    let mask4 = 0b0101_0101;

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

    for i in 0..3 {
        for j in 0..4 {
            matrix[i][j] = temp[i][j];
        }
    }
}

fn compare_allocations(val1: c_int, val2: c_int) -> c_int {
    unsafe {
        let ptr1 = malloc(mem::size_of::<c_int>()) as *mut c_int;
        let ptr2 = malloc(mem::size_of::<c_int>()) as *mut c_int;

        if ptr1.is_null() || ptr2.is_null() {
            free(ptr1.cast());
            free(ptr2.cast());
            return -1;
        }

        *ptr1 = val1;
        *ptr2 = val2;

        let mut result: c_int = if ptr1 < ptr2 {
            1
        } else if ptr1 > ptr2 {
            2
        } else {
            3
        };

        let uninit_ptr = ptr1;
        result += if *uninit_ptr > 0 { 10 } else { 0 };

        free(ptr1.cast());
        free(ptr2.cast());

        result
    }
}

fn arity4(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: ptr::null_mut(),
    };

    let _ = block.label;

    let test_str = *b"Hello\0";
    let empty_str = *b"\0";

    let len1 = process_string(&test_str);
    let len2 = process_string(&empty_str);

    result = result.wrapping_add(len1).wrapping_add(len2);

    shift_array(&mut block.values, 1);

    for i in 0..block.count as usize {
        result = result.wrapping_add(block.values[i]);
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0; 4]; 3];
    init_matrix(&mut matrix);

    result = result
        .wrapping_add(matrix[0][0])
        .wrapping_add(matrix[2][3]);

    let alloc_result = compare_allocations(param1, param2);
    result = result.wrapping_add(alloc_result);

    if param3 != 0 {
        result = result.wrapping_mul(param3) / 100;
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
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
    unsafe {
        let len = len as c_uchar;
        if len < 2 {
            -1
        } else if len == 2 {
            arity2(*params.add(0), *params.add(1))
        } else if len == 3 {
            arity3(*params.add(0), *params.add(1), *params.add(2))
        } else {
            arity4(*params.add(0), *params.add(1), *params.add(2), *params.add(3))
        }
    }
}
