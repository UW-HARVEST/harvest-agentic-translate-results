// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust - preserves byte-identical behavior of original C code.

use std::ffi::c_int;

const NAME_LENGTH: usize = 32;
#[allow(dead_code)]
const MAX_ENTRIES: usize = 10;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

impl DataEntry {
    const fn zeroed() -> Self {
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

/// Mirrors C's strcpy: copies bytes from `src` until the first NUL into `dest`,
/// including the NUL terminator. `dest` must be large enough.
fn c_strcpy(dest: &mut [u8], src: &[u8]) {
    let mut i = 0;
    loop {
        let b = src[i];
        dest[i] = b;
        if b == 0 {
            break;
        }
        i += 1;
    }
}

/// Mirrors C's strcpy from a Rust string literal (no embedded NULs assumed).
fn c_strcpy_str(dest: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        dest[i] = b;
    }
    dest[bytes.len()] = 0;
}

/// Mirrors C's strlen: counts bytes until first NUL.
fn c_strlen(buf: &[u8]) -> usize {
    let mut i = 0;
    while i < buf.len() && buf[i] != 0 {
        i += 1;
    }
    i
}

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<usize> {
    for (i, e) in entries.iter().enumerate() {
        if e.id == target_id {
            return Some(i);
        }
    }
    None
}

/// Returns -1 if `dest` is empty (first byte is NUL, simulating *dest == '\0').
/// Otherwise copies `src` into `dest` and returns its length.
fn process_name(dest: &mut [u8], src: &[u8]) -> c_int {
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }
    c_strcpy(dest, src);
    c_strlen(dest) as c_int
}

fn calculate_lookup(row: usize, col: usize, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp * 2;
        return 1;
    }
    0
}

fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }
    let mut entries = vec![DataEntry::zeroed(); count as usize];
    for i in 0..count {
        let idx = i as usize;
        entries[idx].id = base_id + i;
        entries[idx].value = (base_id + i) * 10;

        // sprintf(temp_name, "Entry_%d", base_id + i);
        let s = format!("Entry_{}", base_id + i);
        let mut temp_name = [0u8; NAME_LENGTH];
        c_strcpy_str(&mut temp_name, &s);

        // strcpy(entries[i].name, temp_name);
        c_strcpy(&mut entries[idx].name, &temp_name);
    }
    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total: c_int = 0;
    for e in entries.iter_mut() {
        let temp_value = e.value;
        if temp_value != 0 {
            // Use wrapping_mul / wrapping_add to mirror C signed overflow behavior
            // (though tests stay in safe ranges).
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

            match entries_opt {
                None => {
                    result = -1;
                }
                Some(entries) => {
                    if count == 0 {
                        // Unreachable given count selection above, but mirror C check.
                        result = -1;
                    } else {
                        let target = 100 + param2;
                        let found = find_entry(&entries, target);
                        match found {
                            None => {
                                result = -2;
                            }
                            Some(idx) => {
                                if entries[idx].id == 0 {
                                    result = -2;
                                } else {
                                    result = entries[idx].value;
                                    // Copy found->name into buffer (unused after).
                                    let name_copy = entries[idx].name;
                                    c_strcpy(&mut buffer, &name_copy);
                                }
                            }
                        }
                    }
                    // entries Vec dropped here (free)
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
                    let total = modify_entries(&mut entries, param2);
                    result = total;
                    if result != 0 {
                        result = result.wrapping_add(param3);
                    }
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let mut lookup_result: c_int = 0;
                let r = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                result = r;
                if result != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default");
            c_strcpy_str(&mut buffer, "Default");
            // result = process_name(buffer, "TestName", NAME_LENGTH);
            let mut src = [0u8; NAME_LENGTH];
            c_strcpy_str(&mut src, "TestName");
            result = process_name(&mut buffer, &src);

            let count = c_strlen(&buffer) as c_int;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
