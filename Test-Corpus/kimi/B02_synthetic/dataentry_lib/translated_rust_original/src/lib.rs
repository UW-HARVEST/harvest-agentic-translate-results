use std::ffi::{c_char, c_int};
use std::os::raw::c_char as RawCChar;

const MAX_ENTRIES: usize = 10;
const NAME_LENGTH: usize = 32;

#[repr(C)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [c_char; NAME_LENGTH],
}

static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<&DataEntry> {
    entries.iter().find(|e| e.id == target_id)
}

fn process_name(dest: &mut [c_char], src: &str) -> c_int {
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }

    let src_bytes = src.as_bytes();
    let len = src_bytes.len().min(dest.len() - 1);

    for (i, &b) in src_bytes.iter().take(len).enumerate() {
        dest[i] = b as c_char;
    }
    dest[len] = 0;

    len as c_int
}

fn calculate_lookup(row: usize, col: usize) -> Option<c_int> {
    let temp = LOOKUP_TABLE.get(row)?.get(col)?;
    if *temp != 0 {
        Some(temp * 2)
    } else {
        None
    }
}

fn create_entries(count: usize, base_id: c_int) -> Option<Vec<DataEntry>> {
    if count == 0 || count > MAX_ENTRIES {
        return None;
    }

    let mut entries = Vec::with_capacity(count);

    for i in 0..count {
        let id = base_id + i as c_int;
        let value = id * 10;
        let name_str = format!("Entry_{}", id);
        let name_bytes = name_str.as_bytes();

        let mut name = [0 as c_char; NAME_LENGTH];
        let copy_len = name_bytes.len().min(NAME_LENGTH - 1);
        for (j, &b) in name_bytes.iter().take(copy_len).enumerate() {
            name[j] = b as c_char;
        }

        entries.push(DataEntry { id, value, name });
    }

    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total = 0;

    for entry in entries.iter_mut() {
        if entry.value != 0 {
            entry.value = entry.value * multiplier;
            total += entry.value;
        }
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: [c_char; NAME_LENGTH] = [0; NAME_LENGTH];

    buffer[0] = 'T' as c_char;
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 as usize } else { 5 };
            let entries = match create_entries(count, 100) {
                Some(e) => e,
                None => return -1,
            };

            let found = find_entry(&entries, 100 + param2);

            match found {
                Some(f) if f.id != 0 => {
                    result = f.value;
                    let name_cstr = unsafe {
                        std::ffi::CStr::from_ptr(f.name.as_ptr())
                    };
                    if let Ok(name_str) = name_cstr.to_str() {
                        let name_bytes = name_str.as_bytes();
                        let copy_len = name_bytes.len().min(NAME_LENGTH - 1);
                        for (i, &b) in name_bytes.iter().take(copy_len).enumerate() {
                            buffer[i] = b as c_char;
                        }
                        buffer[copy_len] = 0;
                    }
                }
                _ => {
                    result = -2;
                }
            }
        }

        2 => {
            let count = if param1 > 0 { param1 as usize } else { 3 };
            let mut entries = match create_entries(count, 200) {
                Some(e) => e,
                None => return -1,
            };

            result = modify_entries(&mut entries, param2);
            if result != 0 {
                result += param3;
            }
        }

        3 => {
            let row = param1 as usize;
            let col = param2 as usize;
            if row < 4 && col < 3 {
                if let Some(lookup_result) = calculate_lookup(row, col) {
                    result = lookup_result + param3;
                }
            }
        }

        _ => {
            let default_str = "Default";
            let default_bytes = default_str.as_bytes();
            let copy_len = default_bytes.len().min(NAME_LENGTH - 1);
            for (i, &b) in default_bytes.iter().take(copy_len).enumerate() {
                buffer[i] = b as c_char;
            }
            buffer[copy_len] = 0;

            result = process_name(&mut buffer, "TestName");

            let count = buffer.iter().position(|&c| c == 0).unwrap_or(NAME_LENGTH);
            if count > 0 {
                result = (count as c_int) * param1;
            }
        }
    }

    result
}
