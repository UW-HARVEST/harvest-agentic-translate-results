use std::ffi::c_int;

const MATRIX: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b00000001;
const FLAG_WRITE: c_int = 0b00000010;
const FLAG_EXECUTE: c_int = 0b00000100;
const FLAG_DELETE: c_int = 0b00001000;

fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled: c_int = if has_read != 0 { 1 } else { 0 };

    let has_write = flags & FLAG_WRITE;
    let write_enabled: c_int = if has_write != 0 { 1 } else { 0 };

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled: c_int = if has_execute != 0 { 1 } else { 0 };

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled: c_int = if has_delete != 0 { 1 } else { 0 };

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    for i in 0..3 {
        for j in 0..4 {
            sum = sum.wrapping_add(MATRIX[i][j]);
        }
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let check1 = param1;
    let valid1: c_int = if check1 != 0 { 1 } else { 0 };

    let check2 = param2;
    let valid2: c_int = if check2 != 0 { 1 } else { 0 };

    let check3 = param3;
    let valid3: c_int = if check3 != 0 { 1 } else { 0 };

    let check4 = param4;
    let valid4: c_int = if check4 != 0 { 1 } else { 0 };

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

    // Replicate dynamic-array behavior: push 4 elements, sum them.
    let mut arr: Vec<c_int> = Vec::with_capacity(2);
    arr.push(param1);
    arr.push(param2);
    arr.push(param3);
    arr.push(param4);

    let mut sum: c_int = 0;
    for v in &arr {
        sum = sum.wrapping_add(*v);
    }

    let flag_count = process_flags(permissions);

    let matrix_sum = calculate_matrix_checksum();

    let result = sum
        .wrapping_mul(hex_multiplier)
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF);

    result
}
