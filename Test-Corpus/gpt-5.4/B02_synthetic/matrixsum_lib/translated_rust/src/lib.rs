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

struct DynamicArray {
    data: Vec<c_int>,
}

fn init_array(initial_capacity: usize) -> Option<DynamicArray> {
    let mut data = Vec::new();
    if data.try_reserve_exact(initial_capacity).is_err() {
        return None;
    }
    Some(DynamicArray { data })
}

fn expand_array(arr: &mut DynamicArray) -> bool {
    let current = arr.data.capacity();
    let new_capacity = if current == 0 { 1 } else { current.saturating_mul(2) };
    if new_capacity <= current {
        return false;
    }
    arr.data.try_reserve_exact(new_capacity - current).is_ok()
}

fn add_element(arr: &mut DynamicArray, value: c_int) -> bool {
    if arr.data.len() >= arr.data.capacity() && !expand_array(arr) {
        return false;
    }
    arr.data.push(value);
    true
}

fn free_array(_arr: DynamicArray) {}

fn process_flags(flags: c_int) -> c_int {
    let read_enabled = ((flags & FLAG_READ) != 0) as c_int;
    let write_enabled = ((flags & FLAG_WRITE) != 0) as c_int;
    let execute_enabled = ((flags & FLAG_EXECUTE) != 0) as c_int;
    let delete_enabled = ((flags & FLAG_DELETE) != 0) as c_int;
    read_enabled + write_enabled + execute_enabled + delete_enabled
}

fn calculate_matrix_checksum() -> c_int {
    MATRIX.iter().flatten().copied().sum()
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let valid1 = (param1 != 0) as c_int;
    let valid2 = (param2 != 0) as c_int;
    let valid3 = (param3 != 0) as c_int;
    let valid4 = (param4 != 0) as c_int;

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
        Some(arr) => arr,
        None => return -1,
    };

    if !add_element(&mut arr, param1)
        || !add_element(&mut arr, param2)
        || !add_element(&mut arr, param3)
        || !add_element(&mut arr, param4)
    {
        free_array(arr);
        return -1;
    }

    let sum: c_int = arr.data.iter().copied().sum();
    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();
    let result = (sum * hex_multiplier) + (flag_count * hex_base) + (matrix_sum & 0xFFF);

    free_array(arr);
    result
}
