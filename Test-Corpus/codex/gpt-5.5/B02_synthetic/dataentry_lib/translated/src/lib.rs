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

fn copy_c_string(dest: &mut [u8; NAME_LENGTH], src: &[u8]) {
    let mut i = 0;
    while i < src.len() && src[i] != 0 {
        dest[i] = src[i];
        i += 1;
    }
    dest[i] = 0;
}

fn c_strlen(buf: &[u8; NAME_LENGTH]) -> c_int {
    buf.iter()
        .position(|&b| b == 0)
        .unwrap_or(NAME_LENGTH) as c_int
}

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<&DataEntry> {
    entries.iter().find(|entry| entry.id == target_id)
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8], _max_len: c_int) -> c_int {
    if dest[0] == 0 {
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

    let mut entries = Vec::with_capacity(count as usize);

    for i in 0..count {
        let id = base_id.wrapping_add(i);
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

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

    for entry in entries {
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
    let mut lookup_result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries = create_entries(count, 100);

            if entries.is_none() || count == 0 {
                result = -1;
            } else {
                let entries = entries.unwrap();
                let found = find_entry(&entries, 100i32.wrapping_add(param2));

                if found.is_none() || found.unwrap().id == 0 {
                    result = -2;
                } else {
                    let found = found.unwrap();
                    result = found.value;
                    copy_c_string(&mut buffer, &found.name);
                }
            }
        }

        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries = create_entries(count, 200);

            if entries.is_none() {
                result = -1;
            } else {
                let mut entries = entries.unwrap();
                result = modify_entries(&mut entries, param2);

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
            result = process_name(&mut buffer, b"TestName\0", NAME_LENGTH as c_int);

            let count = c_strlen(&buffer);
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
