use std::ffi::c_int;

const MAX_ENTRIES: usize = 10;
const NAME_LENGTH: usize = 32;

#[derive(Clone)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

impl DataEntry {
    fn new() -> Self {
        Self {
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
    entries.iter().position(|e| e.id == target_id)
}

/// Reproduces C bug: checks if dest[0] == '\0', but dest was already set to "T".
/// Also ignores max_len (C code ignores it too).
fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8]) -> c_int {
    // C: if (dest == NULL || *dest == '\0') return -1;
    // dest is never NULL here (stack buffer), so only check *dest == '\0'
    if dest[0] == 0 {
        return -1;
    }

    // strcpy(dest, src)
    let src_len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    dest[..src_len].copy_from_slice(&src[..src_len]);
    dest[src_len] = 0;

    // len = strlen(dest)
    src_len as c_int
}

fn calculate_lookup(row: usize, col: usize, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp * 2;
        return 1;
    }
    0
}

/// Reproduces C bug: malloc happens before the NULL/count<=0 check.
/// In Rust we just allocate then check count<=0.
fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    // C: entries = malloc(...); if (entries == NULL || count <= 0) return NULL;
    // malloc of count*sizeof won't fail in normal conditions, so we just check count
    if count <= 0 {
        return None;
    }

    let count = count as usize;
    let mut entries = vec![DataEntry::new(); count];

    for i in 0..count {
        let id = base_id + i as c_int;
        entries[i].id = id;
        entries[i].value = id * 10;

        let formatted = format!("Entry_{}", id);
        let bytes = formatted.as_bytes();
        entries[i].name[..bytes.len()].copy_from_slice(bytes);
        entries[i].name[bytes.len()] = 0;
    }

    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

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
    let mut result: c_int = 0;
    let mut lookup_result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];
    buffer[0] = b'T';

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries_opt = create_entries(count, 100);

            // C: if (entries == NULL || count == 0)
            if entries_opt.is_none() || count == 0 {
                result = -1;
            } else {
                let entries = entries_opt.unwrap();
                let found = find_entry(&entries, 100 + param2);

                if found.is_none() {
                    result = -2;
                } else {
                    let idx = found.unwrap();
                    if entries[idx].id == 0 {
                        result = -2;
                    } else {
                        result = entries[idx].value;
                        // strcpy(buffer, found->name)
                        let name_len = entries[idx]
                            .name
                            .iter()
                            .position(|&b| b == 0)
                            .unwrap_or(NAME_LENGTH);
                        buffer[..name_len].copy_from_slice(&entries[idx].name[..name_len]);
                        buffer[name_len] = 0;
                    }
                }
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries_opt = create_entries(count, 200);

            if entries_opt.is_none() {
                result = -1;
            } else {
                let mut entries = entries_opt.unwrap();
                // C: if ((result = modify_entries(...))) { result += param3; }
                result = modify_entries(&mut entries, param2);
                if result != 0 {
                    result += param3;
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                // C: if ((result = calculate_lookup(...))) { result = lookup_result + param3; }
                result = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                if result != 0 {
                    result = lookup_result + param3;
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default")
            let default_bytes = b"Default";
            buffer[..default_bytes.len()].copy_from_slice(default_bytes);
            buffer[default_bytes.len()] = 0;

            result = process_name(&mut buffer, b"TestName\0");

            // C: if ((count = strlen(buffer))) { result = count * param1; }
            let count = buffer.iter().position(|&b| b == 0).unwrap_or(0) as c_int;
            if count != 0 {
                result = count * param1;
            }
        }
    }

    result
}
