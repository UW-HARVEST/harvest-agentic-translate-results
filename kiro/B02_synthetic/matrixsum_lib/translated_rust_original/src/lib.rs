use std::os::raw::c_int;

static MATRIX: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b00000001;
const FLAG_WRITE: c_int = 0b00000010;
const FLAG_EXECUTE: c_int = 0b00000100;
const FLAG_DELETE: c_int = 0b00001000;

fn process_flags(flags: c_int) -> c_int {
    let read_enabled = ((flags & FLAG_READ) != 0) as c_int;
    let write_enabled = ((flags & FLAG_WRITE) != 0) as c_int;
    let execute_enabled = ((flags & FLAG_EXECUTE) != 0) as c_int;
    let delete_enabled = ((flags & FLAG_DELETE) != 0) as c_int;
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

    let mut permissions: c_int = 0;

    if param1 != 0 { permissions |= FLAG_READ; }
    if param2 != 0 { permissions |= FLAG_WRITE; }
    if param3 != 0 { permissions |= FLAG_EXECUTE; }
    if param4 != 0 { permissions |= FLAG_DELETE; }

    let mut arr = Vec::with_capacity(2);
    arr.push(param1);
    arr.push(param2);
    arr.push(param3);
    arr.push(param4);

    let sum: c_int = arr.iter().copied().fold(0i32, |a, b| a.wrapping_add(b));

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();

    sum.wrapping_mul(hex_multiplier)
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF)
}
