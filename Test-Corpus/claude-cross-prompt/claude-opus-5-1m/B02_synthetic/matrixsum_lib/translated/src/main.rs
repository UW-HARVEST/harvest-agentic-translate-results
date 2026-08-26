// Translated from c_src/src/lib.c — preserves byte-identical behavior.

use std::io::{self, Read};

const FLAG_READ: i32 = 0b00000001;
const FLAG_WRITE: i32 = 0b00000010;
const FLAG_EXECUTE: i32 = 0b00000100;
const FLAG_DELETE: i32 = 0b00001000;

static MATRIX: [[i32; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

struct DynamicArray {
    data: Vec<i32>,
    size: usize,
    capacity: usize,
}

fn init_array(initial_capacity: usize) -> Option<Box<DynamicArray>> {
    // Match C's malloc(initial_capacity * sizeof(int)) — allocate uninitialized
    // memory of that capacity. We model this with a Vec preallocated to capacity
    // and len equal to capacity (filled with zeros so safe Rust is preserved).
    let mut data = Vec::with_capacity(initial_capacity);
    data.resize(initial_capacity, 0);
    Some(Box::new(DynamicArray {
        data,
        size: 0,
        capacity: initial_capacity,
    }))
}

fn expand_array(arr: &mut DynamicArray) -> i32 {
    let new_capacity = arr.capacity * 2;
    arr.data.resize(new_capacity, 0);
    arr.capacity = new_capacity;
    1
}

fn add_element(arr: &mut DynamicArray, value: i32) -> i32 {
    if arr.size >= arr.capacity {
        if expand_array(arr) == 0 {
            return 0;
        }
    }
    arr.data[arr.size] = value;
    arr.size += 1;
    1
}

fn process_flags(flags: i32) -> i32 {
    let has_read = flags & FLAG_READ;
    let read_enabled: i32 = if has_read != 0 { 1 } else { 0 };

    let has_write = flags & FLAG_WRITE;
    let write_enabled: i32 = if has_write != 0 { 1 } else { 0 };

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled: i32 = if has_execute != 0 { 1 } else { 0 };

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled: i32 = if has_delete != 0 { 1 } else { 0 };

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> i32 {
    let mut sum: i32 = 0;
    for i in 0..3 {
        for j in 0..4 {
            sum = sum.wrapping_add(MATRIX[i][j]);
        }
    }
    sum
}

fn matrixsum(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let hex_base: i32 = 0xFF;
    let hex_multiplier: i32 = 0x10;

    let mut permissions: i32 = 0b0000;

    let valid1 = if param1 != 0 { 1 } else { 0 };
    let valid2 = if param2 != 0 { 1 } else { 0 };
    let valid3 = if param3 != 0 { 1 } else { 0 };
    let valid4 = if param4 != 0 { 1 } else { 0 };

    if valid1 != 0 {
        permissions |= FLAG_READ;
    }
    if valid2 != 0 {
        permissions |= FLAG_WRITE;
    }
    if valid3 != 0 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 != 0 {
        permissions |= FLAG_DELETE;
    }

    let mut arr = match init_array(2) {
        Some(a) => a,
        None => return -1,
    };

    add_element(&mut arr, param1);
    add_element(&mut arr, param2);
    add_element(&mut arr, param3);
    add_element(&mut arr, param4);

    let mut sum: i32 = 0;
    for i in 0..arr.size {
        sum = sum.wrapping_add(arr.data[i]);
    }

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();

    (sum.wrapping_mul(hex_multiplier))
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF)
}

/// Reads whitespace-separated integers from stdin (mimicking C's
/// `scanf("%d", ...)` behavior, which reads across newlines).
fn read_all_stdin() -> String {
    let mut buf = String::new();
    let _ = io::stdin().read_to_string(&mut buf);
    buf
}

fn parse_int_token(s: &str) -> Option<i32> {
    // Mimic scanf %d: skip leading whitespace, optional sign, then digits.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let mut neg = false;
    if bytes[i] == b'-' {
        neg = true;
        i += 1;
    } else if bytes[i] == b'+' {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    if start == i {
        return None;
    }
    let s = std::str::from_utf8(&bytes[start..i]).ok()?;
    let v: i64 = s.parse().ok()?;
    let v = if neg { -v } else { v };
    Some(v as i32)
}

fn main() {
    let input = read_all_stdin();
    let mut tokens = input.split_ascii_whitespace();

    let mut nums: [i32; 4] = [0; 4];
    for slot in nums.iter_mut() {
        match tokens.next() {
            Some(t) => match parse_int_token(t) {
                Some(v) => *slot = v,
                None => {
                    *slot = 0;
                }
            },
            None => {
                *slot = 0;
            }
        }
    }

    let result = matrixsum(nums[0], nums[1], nums[2], nums[3]);
    // Match printf("%d\n", result)
    println!("{}", result);
}
