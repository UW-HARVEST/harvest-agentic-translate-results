use std::ffi::c_int;

const NAME_LENGTH: usize = 32;

#[derive(Clone)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

fn find_entry_index(entries: &[DataEntry], target_id: c_int) -> Option<usize> {
    for (i, entry) in entries.iter().enumerate() {
        if entry.id == target_id {
            return Some(i);
        }
    }
    None
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8]) -> c_int {
    // Mirror: if (dest == NULL || *dest == '\0') return -1;
    // dest cannot be null in safe Rust, but we still check the first byte.
    if dest[0] == 0 {
        return -1;
    }
    // Mirror: strcpy(dest, src)
    let mut i = 0usize;
    while i < src.len() && i < NAME_LENGTH {
        let b = src[i];
        dest[i] = b;
        if b == 0 {
            // strcpy stops after the null terminator was copied
            return strlen_buf(dest) as c_int;
        }
        i += 1;
    }
    // If src had no null terminator within range, ensure dest has one
    // (this matches what strcpy would do if src is null-terminated; if not, UB).
    // For the actual call sites, src is always null-terminated.
    strlen_buf(dest) as c_int
}

fn strlen_buf(buf: &[u8; NAME_LENGTH]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(NAME_LENGTH)
}

fn calculate_lookup(row: usize, col: usize, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp.wrapping_mul(2);
        return 1;
    }
    0
}

fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    // Mirror the C check: malloc; if (entries == NULL || count <= 0) return NULL;
    if count <= 0 {
        return None;
    }
    let mut entries: Vec<DataEntry> = Vec::with_capacity(count as usize);
    for i in 0..count {
        let mut name = [0u8; NAME_LENGTH];
        // Mirror: sprintf(temp_name, "Entry_%d", base_id + i);
        let formatted = format!("Entry_{}", base_id.wrapping_add(i));
        let bytes = formatted.as_bytes();
        let len = bytes.len().min(NAME_LENGTH - 1);
        name[..len].copy_from_slice(&bytes[..len]);
        // null terminator already at name[len] since name was zero-initialized
        entries.push(DataEntry {
            id: base_id.wrapping_add(i),
            value: base_id.wrapping_add(i).wrapping_mul(10),
            name,
        });
    }
    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total: c_int = 0;
    for entry in entries.iter_mut() {
        let temp_value = entry.value;
        if temp_value != 0 {
            entry.value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(entry.value);
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
                Some(mut entries) => {
                    if count == 0 {
                        // Unreachable (count is always > 0 here), but mirror C exactly.
                        result = -1;
                    } else {
                        let target = 100i32.wrapping_add(param2);
                        match find_entry_index(&entries, target) {
                            None => {
                                result = -2;
                            }
                            Some(idx) => {
                                let found = &mut entries[idx];
                                if found.id == 0 {
                                    result = -2;
                                } else {
                                    result = found.value;
                                    // strcpy(buffer, found->name) — does not affect return value
                                    let mut i = 0usize;
                                    while i < NAME_LENGTH {
                                        let b = found.name[i];
                                        buffer[i] = b;
                                        if b == 0 {
                                            break;
                                        }
                                        i += 1;
                                    }
                                }
                            }
                        }
                    }
                    // entries dropped here (free)
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
                    result = modify_entries(&mut entries, param2);
                    if result != 0 {
                        result = result.wrapping_add(param3);
                    }
                    // entries dropped here (free)
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let mut lookup_result: c_int = 0;
                result = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                if result != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default")
            let default_str: &[u8] = b"Default";
            for (i, &b) in default_str.iter().enumerate() {
                buffer[i] = b;
            }
            buffer[default_str.len()] = 0;
            // result = process_name(buffer, "TestName", NAME_LENGTH)
            let test_name: &[u8] = b"TestName\0";
            result = process_name(&mut buffer, test_name);
            // if ((count = strlen(buffer)))
            let count = strlen_buf(&buffer) as c_int;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
