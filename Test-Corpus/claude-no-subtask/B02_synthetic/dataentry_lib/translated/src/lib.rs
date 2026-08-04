// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust preserving exact behavior.

use std::ffi::c_int;

const NAME_LENGTH: usize = 32;

#[derive(Clone, Copy)]
struct DataEntry {
    id: c_int,
    value: c_int,
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

static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<usize> {
    for (i, e) in entries.iter().enumerate() {
        if e.id == target_id {
            return Some(i);
        }
    }
    None
}

/// Equivalent of C strcpy length (length of NUL-terminated buffer)
fn cstr_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// Copy src bytes (NUL terminated) into dest, including the trailing NUL.
fn cstr_copy(dest: &mut [u8], src: &[u8]) {
    // Find length of src up to (but not including) NUL
    let n = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    // Copy bytes
    for i in 0..n {
        if i < dest.len() {
            dest[i] = src[i];
        }
    }
    if n < dest.len() {
        dest[n] = 0;
    }
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8]) -> c_int {
    // C: if (dest == NULL || *dest == '\0') return -1;
    // dest is never NULL here, only check *dest == '\0'
    if dest[0] == 0 {
        return -1;
    }
    cstr_copy(dest, src);
    cstr_len(dest) as c_int
}

fn calculate_lookup(row: usize, col: usize, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp * 2;
        1
    } else {
        0
    }
}

/// Returns Some(Vec) for count > 0; None for count <= 0
/// (matches the C semantics: malloc + null check + count<=0 short-circuits to NULL).
fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }
    let count_usize = count as usize;
    let mut entries: Vec<DataEntry> = vec![DataEntry::new(); count_usize];
    let mut temp_name = [0u8; NAME_LENGTH];
    for i in 0..count_usize {
        let i_c = i as c_int;
        entries[i].id = base_id + i_c;
        entries[i].value = (base_id + i_c) * 10;

        // sprintf(temp_name, "Entry_%d", base_id + i)
        // Build into temp_name
        let formatted = format!("Entry_{}", base_id + i_c);
        // zero out temp_name then copy
        for b in temp_name.iter_mut() {
            *b = 0;
        }
        let bytes = formatted.as_bytes();
        let n = bytes.len().min(NAME_LENGTH - 1);
        temp_name[..n].copy_from_slice(&bytes[..n]);
        temp_name[n] = 0;

        // strcpy(entries[i].name, temp_name)
        cstr_copy(&mut entries[i].name, &temp_name);
    }
    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total: c_int = 0;
    for e in entries.iter_mut() {
        let temp_value = e.value;
        if temp_value != 0 {
            e.value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(e.value);
        }
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries_opt = create_entries(count, 100);

            // C: if (entries == NULL || count == 0) { result = -1; break; }
            if entries_opt.is_none() || count == 0 {
                result = -1;
            } else {
                let entries = entries_opt.unwrap();
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
                            cstr_copy(&mut buffer, &entries[idx].name);
                        }
                    }
                }
                // entries dropped here (free)
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries_opt = create_entries(count, 200);

            if entries_opt.is_none() {
                result = -1;
            } else {
                let mut entries = entries_opt.unwrap();
                result = modify_entries(&mut entries, param2);
                if result != 0 {
                    result = result.wrapping_add(param3);
                }
                // free
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let mut lookup_result: c_int = 0;
                result = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default")
            cstr_copy(&mut buffer, b"Default\0");
            result = process_name(&mut buffer, b"TestName\0");
            let count = cstr_len(&buffer) as c_int;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
