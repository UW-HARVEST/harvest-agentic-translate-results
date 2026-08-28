use std::ffi::c_int;

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

fn find_entry(
    entries: &mut [DataEntry],
    count: c_int,
    target_id: c_int,
) -> Option<&mut DataEntry> {
    entries
        .iter_mut()
        .take(count as usize)
        .find(|entry| entry.id == target_id)
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8], _max_len: c_int) -> c_int {
    if dest[0] == 0 {
        return -1;
    }

    let terminator = src.iter().position(|byte| *byte == 0).unwrap_or(src.len());
    dest[..terminator].copy_from_slice(&src[..terminator]);
    dest[terminator] = 0;
    terminator as c_int
}

fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];

    if temp != 0 {
        *result = temp.wrapping_mul(2);
        return 1;
    }

    0
}

fn write_entry_name(dest: &mut [u8; NAME_LENGTH], id: c_int) {
    const PREFIX: &[u8] = b"Entry_";

    dest.fill(0);
    dest[..PREFIX.len()].copy_from_slice(PREFIX);

    let mut digits = [0_u8; 11];
    let mut cursor = digits.len();
    let negative = id < 0;
    let mut magnitude = if negative {
        (id as i64).unsigned_abs()
    } else {
        id as u64
    };

    loop {
        cursor -= 1;
        digits[cursor] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        if magnitude == 0 {
            break;
        }
    }

    let mut offset = PREFIX.len();
    if negative {
        dest[offset] = b'-';
        offset += 1;
    }
    let digit_count = digits.len() - cursor;
    dest[offset..offset + digit_count].copy_from_slice(&digits[cursor..]);
}

fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    if count <= 0 {
        return None;
    }

    let mut entries = Vec::new();
    if entries.try_reserve_exact(count as usize).is_err() {
        return None;
    }

    for i in 0..count {
        let id = base_id.wrapping_add(i);
        let mut entry = DataEntry {
            id,
            value: id.wrapping_mul(10),
            name: [0; NAME_LENGTH],
        };
        write_entry_name(&mut entry.name, id);
        entries.push(entry);
    }

    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], count: c_int, multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

    for entry in entries.iter_mut().take(count as usize) {
        let temp_value = entry.value;
        if temp_value != 0 {
            entry.value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(entry.value);
        }
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(
    mode: c_int,
    param1: c_int,
    param2: c_int,
    param3: c_int,
) -> c_int {
    let mut result: c_int = 0;
    let mut buffer = [0_u8; NAME_LENGTH];
    buffer[0] = b'T';

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let Some(mut entries) = create_entries(count, 100) else {
                return -1;
            };

            if count == 0 {
                result = -1;
            } else {
                let target_id = 100_i32.wrapping_add(param2);
                match find_entry(&mut entries, count, target_id) {
                    None => result = -2,
                    Some(found) if found.id == 0 => result = -2,
                    Some(found) => {
                        result = found.value;
                        buffer.copy_from_slice(&found.name);
                    }
                }
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let Some(mut entries) = create_entries(count, 200) else {
                return -1;
            };

            result = modify_entries(&mut entries, count, param2);
            if result != 0 {
                result = result.wrapping_add(param3);
            }
        }
        3 => {
            if (0..4).contains(&param1) && (0..3).contains(&param2) {
                let mut lookup_result = 0;
                result = calculate_lookup(param1, param2, &mut lookup_result);
                if result != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
        }
        _ => {
            buffer[..8].copy_from_slice(b"Default\0");
            result = process_name(&mut buffer, b"TestName\0", NAME_LENGTH as c_int);

            let count = buffer.iter().position(|byte| *byte == 0).unwrap() as c_int;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
