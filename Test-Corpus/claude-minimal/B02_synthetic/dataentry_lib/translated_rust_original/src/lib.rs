// Copyright 2025 MIT Lincoln Laboratory
// Rust translation of the C library.

use std::os::raw::c_int;

const MAX_ENTRIES: usize = 10;
const NAME_LENGTH: usize = 32;

// Suppress dead code warning for the constant kept for parity with the C source.
#[allow(dead_code)]
const _MAX_ENTRIES_REF: usize = MAX_ENTRIES;

#[derive(Clone, Copy)]
struct DataEntry {
    id: i32,
    value: i32,
    name: [u8; NAME_LENGTH],
}

impl DataEntry {
    fn new() -> Self {
        DataEntry {
            id: 0,
            value: 0,
            name: [0u8; NAME_LENGTH],
        }
    }
}

static LOOKUP_TABLE: [[i32; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

/// Mimics C's strlen on a buffer (length up to first NUL byte).
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// Copy a NUL-terminated source byte string into the destination buffer,
/// always writing a NUL terminator (mimics C's strcpy where the destination
/// is large enough).
fn c_strcpy(dest: &mut [u8], src: &[u8]) {
    let len = c_strlen(src);
    let copy_len = len.min(dest.len().saturating_sub(1));
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    if copy_len < dest.len() {
        dest[copy_len] = 0;
    }
}

fn find_entry(entries: &[DataEntry], target_id: i32) -> Option<usize> {
    for (i, e) in entries.iter().enumerate() {
        if e.id == target_id {
            return Some(i);
        }
    }
    None
}

fn process_name(dest: &mut [u8], src: &[u8]) -> i32 {
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }
    c_strcpy(dest, src);
    c_strlen(dest) as i32
}

fn calculate_lookup(row: usize, col: usize, result: &mut i32) -> i32 {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp * 2;
        return 1;
    }
    0
}

fn create_entries(count: i32, base_id: i32) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }
    let count_usize = count as usize;
    let mut entries: Vec<DataEntry> = vec![DataEntry::new(); count_usize];

    let mut temp_name = [0u8; NAME_LENGTH];

    for i in 0..count_usize {
        entries[i].id = base_id + i as i32;
        entries[i].value = (base_id + i as i32) * 10;

        // sprintf(temp_name, "Entry_%d", base_id + i);
        let formatted = format!("Entry_{}", base_id + i as i32);
        let bytes = formatted.as_bytes();
        // Reset temp_name
        for b in temp_name.iter_mut() {
            *b = 0;
        }
        let copy_len = bytes.len().min(NAME_LENGTH - 1);
        temp_name[..copy_len].copy_from_slice(&bytes[..copy_len]);
        temp_name[copy_len] = 0;

        // strcpy(entries[i].name, temp_name);
        c_strcpy(&mut entries[i].name, &temp_name);
    }

    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: i32) -> i32 {
    let mut total: i32 = 0;
    for entry in entries.iter_mut() {
        let temp_value = entry.value;
        if temp_value != 0 {
            entry.value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(entry.value);
        }
    }
    total
}

#[no_mangle]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: i32 = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries_opt = create_entries(count, 100);

            match entries_opt {
                None => {
                    result = -1;
                }
                Some(entries) => {
                    if count == 0 {
                        result = -1;
                    } else {
                        let found_idx = find_entry(&entries, 100 + param2);
                        match found_idx {
                            None => {
                                result = -2;
                            }
                            Some(idx) => {
                                if entries[idx].id == 0 {
                                    result = -2;
                                } else {
                                    result = entries[idx].value;
                                    let name_copy = entries[idx].name;
                                    c_strcpy(&mut buffer, &name_copy);
                                }
                            }
                        }
                    }
                    // Vec dropped here, mimicking free
                    drop(entries);
                }
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries_opt = create_entries(count, 200);

            match entries_opt {
                None => {
                    result = -1;
                }
                Some(mut entries) => {
                    let modified = modify_entries(&mut entries, param2);
                    result = modified;
                    if result != 0 {
                        result = result.wrapping_add(param3);
                    }
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let mut lookup_result: i32 = 0;
                let calc = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                result = calc;
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            c_strcpy(&mut buffer, b"Default\0");
            result = process_name(&mut buffer, b"TestName\0");

            let count = c_strlen(&buffer) as i32;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    // Touch buffer to keep its semantic side-effects (matches C source).
    let _ = buffer;

    result as c_int
}
