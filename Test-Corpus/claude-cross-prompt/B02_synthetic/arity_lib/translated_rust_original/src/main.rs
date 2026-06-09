// Translated from c_src/src/lib.c
// The original C source is a shared library (no `main`). This executable
// has no top-level behavior and therefore produces no output, matching the
// behavior of compiling the C library (which also produces no output).

use std::alloc::{alloc, dealloc, Layout};

#[allow(dead_code)]
struct DataBlock {
    values: [i32; 4],
    count: i32,
    label: Option<*mut u8>,
}

fn shift_array(arr: &mut [i32], size: i32, positions: i32) {
    if positions > 0 && positions < size {
        // memmove(arr + positions, arr, (size - positions) * sizeof(int));
        let size = size as usize;
        let positions = positions as usize;
        // Move elements arr[0..size-positions] -> arr[positions..size]
        // Use copy_within which handles overlap.
        arr.copy_within(0..(size - positions), positions);
        for i in 0..positions {
            arr[i] = 0;
        }
    }
}

fn process_string(s: &[u8]) -> i32 {
    // The C code does: if (*str) { return strlen(str); } return 0;
    // The byte slice here represents a C-style string (without the trailing NUL).
    if !s.is_empty() && s[0] != 0 {
        // strlen up to the first NUL
        let mut len = 0usize;
        while len < s.len() && s[len] != 0 {
            len += 1;
        }
        len as i32
    } else {
        0
    }
}

fn apply_bitmask(value: i32, operation: i32) -> i32 {
    let mask1: i32 = 0b1111_0000;
    let mask2: i32 = 0b0000_1111;
    let mask3: i32 = 0b1010_1010;
    let mask4: i32 = 0b0101_0101;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

fn init_matrix(matrix: &mut [[i32; 4]; 3]) {
    let temp: [[i32; 4]; 3] = [
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

fn compare_allocations(val1: i32, val2: i32) -> i32 {
    // Faithful translation: allocate two ints with the system allocator,
    // compare addresses, and read back via the "uninit_ptr" alias of ptr1.
    unsafe {
        let layout = Layout::new::<i32>();
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

fn arity4(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result: i32 = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: None,
    };

    let test_str: &[u8] = b"Hello\0";
    let empty_str: &[u8] = b"\0";

    let len1 = process_string(test_str);
    let len2 = process_string(empty_str);

    result = result.wrapping_add(len1.wrapping_add(len2));

    shift_array(&mut block.values, 4, 1);

    for i in 0..(block.count as usize) {
        result = result.wrapping_add(block.values[i]);
    }

    // C's `%` for negative operands matches Rust's `%` (truncated toward zero).
    result = apply_bitmask(result, param1 % 4);

    let mut matrix: [[i32; 4]; 3] = [[0; 4]; 3];
    init_matrix(&mut matrix);

    result = result.wrapping_add(matrix[0][0].wrapping_add(matrix[2][3]));

    let alloc_result = compare_allocations(param1, param2);
    result = result.wrapping_add(alloc_result);

    if param3 != 0 {
        // (result * param3) / 100, with C's wrapping/truncation semantics
        result = result.wrapping_mul(param3) / 100;
    }

    if param4 != 0 {
        result = result.wrapping_add(param4);
    }

    result
}

fn arity2(p1: i32, p2: i32) -> i32 {
    arity4(p1, p2, 0, 0)
}

fn arity3(p1: i32, p2: i32, p3: i32) -> i32 {
    arity4(p1, p2, p3, 0)
}

#[allow(dead_code)]
fn arity(len: u8, params: &[i32]) -> i32 {
    if len < 2 {
        -1
    } else if len == 2 {
        arity2(params[0], params[1])
    } else if len == 3 {
        arity3(params[0], params[1], params[2])
    } else {
        arity4(params[0], params[1], params[2], params[3])
    }
}

fn main() {
    // The original C source defines a library only (no `main`). To match
    // its byte-identical (empty) output, this executable does nothing.
}
