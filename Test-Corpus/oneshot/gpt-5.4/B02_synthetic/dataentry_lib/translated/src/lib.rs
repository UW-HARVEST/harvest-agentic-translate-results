use std::os::raw::c_int;

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

fn find_entry(entries: &mut [DataEntry], target_id: c_int) -> Option<&mut DataEntry> {
    entries.iter_mut().find(|entry| entry.id == target_id)
}

fn write_c_string(dest: &mut [u8], src: &str) {
    if dest.is_empty() {
        return;
    }
    let bytes = src.as_bytes();
    let len = bytes.len().min(dest.len().saturating_sub(1));
    dest[..len].copy_from_slice(&bytes[..len]);
    dest[len] = 0;
    for b in &mut dest[len + 1..] {
        *b = 0;
    }
}

fn c_string_len(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn process_name(dest: &mut [u8], src: &str, _max_len: c_int) -> c_int {
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }
    write_c_string(dest, src);
    c_string_len(dest) as c_int
}

fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];
    if temp != 0 {
        *result = temp * 2;
        1
    } else {
        0
    }
}

fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }

    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count {
        let id = base_id + i;
        let mut name = [0u8; NAME_LENGTH];
        write_c_string(&mut name, &format!("Entry_{}", id));
        entries.push(DataEntry {
            id,
            value: id * 10,
            name,
        });
    }
    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
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
    let mut count: c_int;
    let mut lookup_result = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            count = if param1 > 0 { param1 } else { 5 };
            let mut entries = match create_entries(count, 100) {
                Some(entries) => entries,
                None => return -1,
            };

            match find_entry(&mut entries, 100 + param2) {
                Some(found) if found.id != 0 => {
                    result = found.value;
                    let len = c_string_len(&found.name);
                    buffer[..len].copy_from_slice(&found.name[..len]);
                    if len < buffer.len() {
                        buffer[len] = 0;
                    }
                }
                _ => {
                    result = -2;
                }
            }
        }
        2 => {
            count = if param1 > 0 { param1 } else { 3 };
            let mut entries = match create_entries(count, 200) {
                Some(entries) => entries,
                None => return -1,
            };

            result = modify_entries(&mut entries, param2);
            if result != 0 {
                result += param3;
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                result = calculate_lookup(param1, param2, &mut lookup_result);
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            write_c_string(&mut buffer, "Default");
            result = process_name(&mut buffer, "TestName", NAME_LENGTH as c_int);
            count = c_string_len(&buffer) as c_int;
            if count != 0 {
                result = count * param1;
            }
        }
    }

    result
}
