use std::ffi::c_int;

const MAX_ENTRIES: usize = 10;
const NAME_LENGTH: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

const LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

fn c_strlen(bytes: &[u8]) -> c_int {
    bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len()) as c_int
}

fn copy_c_string(dest: &mut [u8], src: &[u8]) {
    let len = src.iter().position(|&byte| byte == 0).unwrap_or(src.len());
    let copy_len = len.min(dest.len().saturating_sub(1));
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    if copy_len < dest.len() {
        dest[copy_len] = 0;
    }
}

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<&DataEntry> {
    let mut index = 0usize;
    let end = entries.len();

    while index < end {
        if entries[index].id == target_id {
            return Some(&entries[index]);
        }
        index += 1;
    }

    None
}

fn process_name(dest: Option<&mut [u8]>, src: &[u8], _max_len: c_int) -> c_int {
    let Some(dest) = dest else {
        return -1;
    };

    if dest.first().copied().unwrap_or(0) == 0 {
        return -1;
    }

    copy_c_string(dest, src);
    c_strlen(dest)
}

fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];

    if temp != 0 {
        *result = temp.wrapping_mul(2);
        return 1;
    }

    0
}

fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }

    let count = usize::try_from(count).ok()?;
    let mut entries = Vec::new();
    entries.try_reserve_exact(count).ok()?;

    for i in 0..count {
        let id = base_id.wrapping_add(i as c_int);
        let mut name = [0u8; NAME_LENGTH];
        let temp_name = format!("Entry_{id}");
        copy_c_string(&mut name, temp_name.as_bytes());

        entries.push(DataEntry {
            id,
            value: id.wrapping_mul(10),
            name,
        });
    }

    Some(entries)
}

fn modify_entries(entries: Option<&mut [DataEntry]>, multiplier: c_int) -> c_int {
    let Some(entries) = entries else {
        return -1;
    };

    let mut index = 0usize;
    let mut total: c_int = 0;

    while index < entries.len() {
        let temp_value = entries[index].value;
        if temp_value != 0 {
            entries[index].value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(entries[index].value);
        }
        index += 1;
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result = 0;
    let count;
    let mut lookup_result = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    let _ = MAX_ENTRIES;

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            count = if param1 > 0 { param1 } else { 5 };
            let entries = create_entries(count, 100);

            if entries.is_none() || count == 0 {
                result = -1;
            } else if let Some(entries) = entries {
                let found = find_entry(&entries, 100i32.wrapping_add(param2));

                if found.is_none() || found.is_some_and(|entry| entry.id == 0) {
                    result = -2;
                } else if let Some(found) = found {
                    result = found.value;
                    copy_c_string(&mut buffer, &found.name);
                }
            }
        }
        2 => {
            count = if param1 > 0 { param1 } else { 3 };
            let mut entries = create_entries(count, 200);

            if entries.is_none() {
                result = -1;
            } else if let Some(entries) = entries.as_mut() {
                result = modify_entries(Some(entries.as_mut_slice()), param2);
                if result != 0 {
                    result = result.wrapping_add(param3);
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                result = calculate_lookup(param1, param2, &mut lookup_result);
                if result != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
        }
        _ => {
            copy_c_string(&mut buffer, b"Default\0");
            result = process_name(Some(&mut buffer), b"TestName\0", NAME_LENGTH as c_int);

            count = c_strlen(&buffer);
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
