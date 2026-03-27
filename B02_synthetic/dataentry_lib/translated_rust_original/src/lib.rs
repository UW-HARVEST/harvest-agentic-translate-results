use std::ffi::c_int;

#[allow(dead_code)]
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

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<&DataEntry> {
    entries.iter().find(|e| e.id == target_id)
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8]) -> c_int {
    // C checks: dest == NULL || *dest == '\0'
    // dest is never NULL here; check if first byte is '\0'
    if dest[0] == 0 {
        return -1;
    }
    // strcpy(dest, src)
    let len = src.iter().position(|&b| b == 0).unwrap_or(src.len());
    dest[..len].copy_from_slice(&src[..len]);
    dest[len] = 0;
    len as c_int
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
    // C: malloc first, then check count <= 0
    // malloc(count * sizeof(DataEntry)) with count <= 0 is UB/implementation-defined
    // In Rust, we replicate: if allocation "succeeds" but count <= 0, return None
    if count <= 0 {
        return None;
    }

    let count = count as usize;
    let mut entries = vec![DataEntry::new(); count];

    for i in 0..count {
        let id = base_id + i as c_int;
        entries[i].id = id;
        entries[i].value = id * 10;

        let mut tmp = [0u8; NAME_LENGTH];
        let cursor = &mut tmp[..];
        // sprintf(temp_name, "Entry_%d", base_id + i)
        let s = format!("Entry_{}", id);
        let bytes = s.as_bytes();
        cursor[..bytes.len()].copy_from_slice(bytes);
        cursor[bytes.len()] = 0;

        // strcpy(entries[i].name, temp_name)
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
#[allow(unused_assignments)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];
    buffer[0] = b'T';
    // buffer[1] already 0

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries = create_entries(count, 100);

            if entries.is_none() || count == 0 {
                return -1;
            }
            let entries = entries.unwrap();

            let target_id = 100 + param2;
            match find_entry(&entries, target_id) {
                None => {
                    result = -2;
                }
                Some(found) => {
                    if found.id == 0 {
                        result = -2;
                    } else {
                        result = found.value;
                        // strcpy(buffer, found->name)
                        buffer = found.name;
                    }
                }
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries = create_entries(count, 200);

            if entries.is_none() {
                return -1;
            }
            let mut entries = entries.unwrap();

            result = modify_entries(&mut entries, param2);
            if result != 0 {
                result += param3;
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
            let default_bytes = b"Default";
            buffer[..default_bytes.len()].copy_from_slice(default_bytes);
            buffer[default_bytes.len()] = 0;

            let src = b"TestName\0";
            result = process_name(&mut buffer, src);

            // count = strlen(buffer)
            let count = buffer.iter().position(|&b| b == 0).unwrap_or(NAME_LENGTH);
            if count != 0 {
                result = count as c_int * param1;
            }
        }
    }

    result
}
