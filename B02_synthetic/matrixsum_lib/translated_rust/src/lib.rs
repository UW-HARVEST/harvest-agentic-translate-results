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
    let read_enabled = c_int::from(flags & FLAG_READ != 0);
    let write_enabled = c_int::from(flags & FLAG_WRITE != 0);
    let execute_enabled = c_int::from(flags & FLAG_EXECUTE != 0);
    let delete_enabled = c_int::from(flags & FLAG_DELETE != 0);
    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;
    for row in &MATRIX {
        for &val in row {
            sum += val;
        }
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    if param1 != 0 { permissions |= FLAG_READ; }
    if param2 != 0 { permissions |= FLAG_WRITE; }
    if param3 != 0 { permissions |= FLAG_EXECUTE; }
    if param4 != 0 { permissions |= FLAG_DELETE; }

    // Replicate DynamicArray behavior: collect params, sum them
    let arr = vec![param1, param2, param3, param4];
    let sum: c_int = arr.iter().sum();

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();

    (sum * hex_multiplier) + (flag_count * hex_base) + (matrix_sum & 0xFFF)
}
