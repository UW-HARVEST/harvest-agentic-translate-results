use std::ffi::c_int;

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
    for e in entries {
        if e.id == target_id {
            return Some(e);
        }
    }
    None
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8]) -> c_int {
    // C checks: if (dest == NULL || *dest == '\0') return -1;
    // dest is never NULL here, so only check *dest == '\0'
    if dest[0] == 0 {
        return -1;
    }

    // strcpy(dest, src) - copy src into dest
    strcpy_buf(dest, src);

    // len = strlen(dest)
    let len = strlen_buf(dest);
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
    // C: malloc first, THEN check NULL || count <= 0
    // malloc(count * sizeof) with count <= 0 is UB in C; we mimic the
    // post-malloc guard: if (entries == NULL || count <= 0) return NULL;
    if count <= 0 {
        return None;
    }

    let mut entries = vec![DataEntry::new(); count as usize];

    for i in 0..count as usize {
        entries[i].id = base_id + i as c_int;
        entries[i].value = (base_id + i as c_int) * 10;

        let mut temp_name = [0u8; NAME_LENGTH];
        // sprintf(temp_name, "Entry_%d", base_id + i)
        let s = format!("Entry_{}", base_id + i as c_int);
        let bytes = s.as_bytes();
        temp_name[..bytes.len()].copy_from_slice(bytes);

        // strcpy(entries[i].name, temp_name)
        strcpy_buf(&mut entries[i].name, &temp_name);
    }

    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

    for e in entries.iter_mut() {
        let temp_value = e.value;
        if temp_value != 0 {
            e.value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(e.value);
        }
    }

    total
}

// Helper: strcpy semantics - copy bytes up to and including first NUL
fn strcpy_buf(dest: &mut [u8; NAME_LENGTH], src: &[u8]) {
    for i in 0..NAME_LENGTH {
        if i < src.len() {
            dest[i] = src[i];
            if src[i] == 0 {
                break;
            }
        } else {
            dest[i] = 0;
            break;
        }
    }
}

// Helper: strlen semantics
fn strlen_buf(buf: &[u8; NAME_LENGTH]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(NAME_LENGTH)
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut lookup_result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries_opt = create_entries(count, 100);

            if entries_opt.is_none() || count == 0 {
                result = -1;
            } else {
                let entries = entries_opt.unwrap();
                let found = find_entry(&entries, 100 + param2);

                if found.is_none() || found.unwrap().id == 0 {
                    result = -2;
                } else {
                    let f = found.unwrap();
                    result = f.value;
                    strcpy_buf(&mut buffer, &f.name);
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
                result = modify_entries(&mut entries, param2);
                // C: if ((result = modify_entries(...))) { result += param3; }
                if result != 0 {
                    result = result.wrapping_add(param3);
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                result = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                // C: if ((result = calculate_lookup(...))) { result = lookup_result + param3; }
                if result != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default")
            strcpy_buf(&mut buffer, b"Default\0");
            result = process_name(&mut buffer, b"TestName\0");

            // C: if ((count = strlen(buffer))) { result = count * param1; }
            let count = strlen_buf(&buffer) as c_int;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
