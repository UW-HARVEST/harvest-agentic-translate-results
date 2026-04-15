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

pub struct DynamicArray {
    pub data: Vec<c_int>,
}

fn init_array(initial_capacity: usize) -> Option<Box<DynamicArray>> {
    Some(Box::new(DynamicArray {
        data: Vec::with_capacity(initial_capacity),
    }))
}

fn expand_array(_arr: &mut DynamicArray) -> c_int {
    1
}

fn add_element(arr: &mut DynamicArray, value: c_int) -> c_int {
    arr.data.push(value);
    1
}

fn free_array(_arr: Option<Box<DynamicArray>>) {}

fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled = if has_read != 0 { 1 } else { 0 };

    let has_write = flags & FLAG_WRITE;
    let write_enabled = if has_write != 0 { 1 } else { 0 };

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = if has_execute != 0 { 1 } else { 0 };

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = if has_delete != 0 { 1 } else { 0 };

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> c_int {
    let mut sum = 0;

    for i in 0..3 {
        for j in 0..4 {
            sum += MATRIX[i][j];
        }
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base = 0xFF;
    let hex_multiplier = 0x10;

    let mut permissions = 0b0000;

    let check1 = param1;
    let valid1 = if check1 != 0 { 1 } else { 0 };

    let check2 = param2;
    let valid2 = if check2 != 0 { 1 } else { 0 };

    let check3 = param3;
    let valid3 = if check3 != 0 { 1 } else { 0 };

    let check4 = param4;
    let valid4 = if check4 != 0 { 1 } else { 0 };

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

    let mut sum = 0;
    for i in 0..arr.data.len() {
        sum += arr.data[i];
    }

    let flag_count = process_flags(permissions);

    let matrix_sum = calculate_matrix_checksum();

    let result = (sum * hex_multiplier) + (flag_count * hex_base) + (matrix_sum & 0xFFF);

    free_array(Some(arr));

    result
}
