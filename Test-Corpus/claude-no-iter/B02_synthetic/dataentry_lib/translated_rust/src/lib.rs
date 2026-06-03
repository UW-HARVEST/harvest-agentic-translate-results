// Translated from C to Rust. Preserves exact behavior of the original C code.

use std::ffi::c_int;

const NAME_LENGTH: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
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

fn find_entry(entries: &[DataEntry], target_id: c_int) -> Option<usize> {
    for (i, e) in entries.iter().enumerate() {
        if e.id == target_id {
            return Some(i);
        }
    }
    None
}

fn process_name(dest: &mut [u8; NAME_LENGTH], src: &[u8]) -> c_int {
    // Mirrors process_name in C: returns -1 if dest is NULL or first byte is 0.
    // dest is never NULL in Rust references; check first byte.
    if dest[0] == 0 {
        return -1;
    }
    // strcpy(dest, src)
    cstrcpy(dest, src);
    // strlen(dest)
    cstrlen(dest) as c_int
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
    // In C: malloc happens first, then `if (entries == NULL || count <= 0) return NULL;`
    // For our purposes, treat count <= 0 as failure (returns NULL/None).
    if count <= 0 {
        return None;
    }
    let mut entries: Vec<DataEntry> = Vec::with_capacity(count as usize);
    for i in 0..count {
        let mut name = [0u8; NAME_LENGTH];
        let id_val = base_id.wrapping_add(i);
        // sprintf(temp_name, "Entry_%d", base_id + i);
        let s = format!("Entry_{}", id_val);
        let bytes = s.as_bytes();
        // strcpy into name (assumes fits; C version assumes the same)
        let n = bytes.len().min(NAME_LENGTH - 1);
        name[..n].copy_from_slice(&bytes[..n]);
        // null terminator already in place
        entries.push(DataEntry {
            id: id_val,
            value: id_val.wrapping_mul(10),
            name,
        });
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

fn cstrlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

fn cstrcpy(dest: &mut [u8], src: &[u8]) {
    // Copy bytes from src up to (and including) the first NUL or end-of-src,
    // then ensure a NUL terminator within dest if there is room.
    let src_len = cstrlen(src).min(src.len());
    let n = src_len.min(dest.len());
    dest[..n].copy_from_slice(&src[..n]);
    if n < dest.len() {
        dest[n] = 0;
    }
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
            let count: c_int = if param1 > 0 { param1 } else { 5 };
            let entries_opt = create_entries(count, 100);

            match entries_opt {
                None => {
                    result = -1;
                }
                Some(entries) => {
                    if count == 0 {
                        // Unreachable in practice (count > 0 above), but preserve check
                        result = -1;
                    } else {
                        let target = 100i32.wrapping_add(param2);
                        let found_idx = find_entry(&entries, target);

                        match found_idx {
                            None => {
                                result = -2;
                            }
                            Some(idx) => {
                                if entries[idx].id == 0 {
                                    result = -2;
                                } else {
                                    result = entries[idx].value;
                                    // strcpy(buffer, found->name)
                                    let name = entries[idx].name;
                                    cstrcpy(&mut buffer, &name);
                                }
                            }
                        }
                    }
                    // entries dropped (free) here
                    drop(entries);
                }
            }
        }
        2 => {
            let count: c_int = if param1 > 0 { param1 } else { 3 };
            let entries_opt = create_entries(count, 200);

            match entries_opt {
                None => {
                    result = -1;
                }
                Some(mut entries) => {
                    let r = modify_entries(&mut entries, param2);
                    result = r;
                    if r != 0 {
                        result = r.wrapping_add(param3);
                    }
                    drop(entries);
                }
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let r = calculate_lookup(param1, param2, &mut lookup_result);
                result = r;
                if r != 0 {
                    result = lookup_result.wrapping_add(param3);
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default");
            cstrcpy(&mut buffer, b"Default\0");
            // result = process_name(buffer, "TestName", NAME_LENGTH);
            result = process_name(&mut buffer, b"TestName\0");
            // if ((count = strlen(buffer))) result = count * param1;
            let count = cstrlen(&buffer) as c_int;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}
