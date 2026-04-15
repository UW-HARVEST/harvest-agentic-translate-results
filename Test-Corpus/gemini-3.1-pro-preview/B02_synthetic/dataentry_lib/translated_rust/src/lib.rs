use std::os::raw::c_int;

const NAME_LENGTH: usize = 32;

#[derive(Clone, Copy)]
struct DataEntry {
    id: i32,
    value: i32,
    name: [u8; NAME_LENGTH],
}

static LOOKUP_TABLE: [[i32; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

fn find_entry(entries: &[DataEntry], target_id: i32) -> Option<&DataEntry> {
    entries.iter().find(|&e| e.id == target_id)
}

fn process_name(dest: &mut [u8], src: &[u8]) -> i32 {
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }

    let src_len = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    let copy_len = src_len.min(dest.len() - 1);
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    dest[copy_len] = 0;

    copy_len as i32
}

fn calculate_lookup(row: usize, col: usize, result: &mut i32) -> i32 {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp * 2;
        1
    } else {
        0
    }
}

fn create_entries(count: i32, base_id: i32) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }
    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        let mut name = [0u8; NAME_LENGTH];
        let s = format!("Entry_{}", base_id + i);
        let bytes = s.as_bytes();
        let copy_len = bytes.len().min(NAME_LENGTH - 1);
        name[..copy_len].copy_from_slice(&bytes[..copy_len]);
        name[copy_len] = 0;

        entries.push(DataEntry {
            id: base_id + i,
            value: (base_id + i) * 10,
            name,
        });
    }
    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: i32) -> i32 {
    let mut total = 0;
    for entry in entries.iter_mut() {
        let temp_value = entry.value;
        if temp_value != 0 {
            entry.value = temp_value * multiplier;
            total += entry.value;
        }
    }
    total
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result = 0;
    let mut buffer = [0u8; NAME_LENGTH];
    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            if let Some(entries) = create_entries(count, 100) {
                if let Some(found) = find_entry(&entries, 100 + param2) {
                    if found.id == 0 {
                        result = -2;
                    } else {
                        result = found.value;
                        let name_len = found.name.iter().position(|&c| c == 0).unwrap_or(found.name.len());
                        let copy_len = name_len.min(buffer.len() - 1);
                        buffer[..copy_len].copy_from_slice(&found.name[..copy_len]);
                        buffer[copy_len] = 0;
                    }
                } else {
                    result = -2;
                }
            } else {
                result = -1;
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            if let Some(mut entries) = create_entries(count, 200) {
                result = modify_entries(&mut entries, param2);
                if result != 0 {
                    result += param3;
                }
            } else {
                result = -1;
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let mut lookup_result = 0;
                result = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            let default_str = b"Default";
            let copy_len = default_str.len().min(buffer.len() - 1);
            buffer[..copy_len].copy_from_slice(&default_str[..copy_len]);
            buffer[copy_len] = 0;

            result = process_name(&mut buffer, b"TestName\0");

            let count = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len()) as i32;
            if count != 0 {
                result = count * param1;
            }
        }
    }

    result
}
