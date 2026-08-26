// Translated from c_src/src/lib.c.
//
// The C source defines a single library entry point `dataentry` and contains
// no `main`. This Rust program wraps `dataentry` in an executable that reads
// four integers from stdin (mimicking C's `scanf("%d %d %d %d", ...)`,
// which skips arbitrary whitespace including newlines) and prints the
// returned int with `printf("%d\n", result)` semantics.

use std::io::{self, Read, Write};

const NAME_LENGTH: usize = 32;

#[derive(Clone)]
struct DataEntry {
    id: i32,
    value: i32,
    name: [u8; NAME_LENGTH],
}

impl DataEntry {
    fn new() -> Self {
        DataEntry {
            id: 0,
            value: 0,
            name: [0u8; NAME_LENGTH],
        }
    }
}

static LOOKUP_TABLE: [[i32; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

// C: strlen — counts bytes up to first NUL.
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

// C: strcpy(dest, src) — copies bytes from `src` until and including its
// terminating NUL into `dest`. We assume `dest` is large enough, matching the
// C code's usage.
fn c_strcpy(dest: &mut [u8], src: &[u8]) {
    let len = c_strlen(src);
    dest[..len].copy_from_slice(&src[..len]);
    if dest.len() > len {
        dest[len] = 0;
    }
}

// C: sprintf(temp_name, "Entry_%d", n)
fn sprintf_entry(dest: &mut [u8], n: i32) {
    let s = format!("Entry_{}", n);
    let bytes = s.as_bytes();
    dest[..bytes.len()].copy_from_slice(bytes);
    dest[bytes.len()] = 0;
}

fn find_entry(entries: &mut [DataEntry], count: usize, target_id: i32) -> Option<usize> {
    let mut i = 0usize;
    while i < count {
        if entries[i].id == target_id {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn process_name(dest: &mut [u8], src: &[u8], _max_len: usize) -> i32 {
    // The original C code performs `if (dest == NULL || *dest == '\0')`.
    // We always have a non-null buffer here, so only the empty-string check
    // applies.
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }
    c_strcpy(dest, src);
    c_strlen(dest) as i32
}

fn calculate_lookup(row: usize, col: usize, result: &mut i32) -> i32 {
    let temp = LOOKUP_TABLE[row][col];
    if temp != 0 {
        *result = temp * 2;
        return 1;
    }
    0
}

fn create_entries(count: i32, base_id: i32) -> Option<Vec<DataEntry>> {
    // The C code calls malloc(count * sizeof(DataEntry)) and then checks for
    // NULL or count <= 0. With count <= 0, malloc with a non-positive product
    // typically still returns a non-null pointer, and the code returns NULL.
    // We mirror this by short-circuiting when count <= 0.
    if count <= 0 {
        return None;
    }
    let mut entries: Vec<DataEntry> = vec![DataEntry::new(); count as usize];
    let mut temp_name = [0u8; NAME_LENGTH];
    for i in 0..count {
        let idx = i as usize;
        entries[idx].id = base_id + i;
        entries[idx].value = (base_id + i) * 10;
        // Reset temp_name (sprintf overwrites from start).
        for b in temp_name.iter_mut() {
            *b = 0;
        }
        sprintf_entry(&mut temp_name, base_id + i);
        c_strcpy(&mut entries[idx].name, &temp_name);
    }
    Some(entries)
}

fn modify_entries(entries: &mut [DataEntry], count: usize, multiplier: i32) -> i32 {
    let mut total: i32 = 0;
    let mut i = 0usize;
    while i < count {
        let temp_value = entries[i].value;
        if temp_value != 0 {
            // Use wrapping arithmetic to mirror C's signed-int overflow
            // behavior (which is technically UB in C, but in practice wraps
            // on two's-complement targets used by the original program).
            let new_val = temp_value.wrapping_mul(multiplier);
            entries[i].value = new_val;
            total = total.wrapping_add(new_val);
        }
        i += 1;
    }
    total
}

fn dataentry(mode: i32, param1: i32, param2: i32, param3: i32) -> i32 {
    let mut result: i32 = 0;
    let mut buffer = [0u8; NAME_LENGTH];
    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            let count = if param1 > 0 { param1 } else { 5 };
            let entries_opt = create_entries(count, 100);
            // Match C: `if (entries == NULL || count == 0) { result = -1; break; }`.
            // Because create_entries returns None when count <= 0, we treat
            // that as the NULL branch.
            if entries_opt.is_none() || count == 0 {
                result = -1;
            } else {
                let mut entries = entries_opt.unwrap();
                let count_usize = count as usize;
                let found = find_entry(&mut entries, count_usize, 100 + param2);
                match found {
                    None => {
                        result = -2;
                    }
                    Some(idx) => {
                        // C also checks `found->id == 0`, but the entry's id
                        // matched target_id to be returned, so this only
                        // triggers when target_id == 0.
                        if entries[idx].id == 0 {
                            result = -2;
                        } else {
                            result = entries[idx].value;
                            // Copy name into buffer.
                            let name_copy = entries[idx].name;
                            c_strcpy(&mut buffer, &name_copy);
                        }
                    }
                }
                // free(entries) — Vec is dropped here.
                drop(entries);
            }
        }
        2 => {
            let count = if param1 > 0 { param1 } else { 3 };
            let entries_opt = create_entries(count, 200);
            if entries_opt.is_none() {
                result = -1;
            } else {
                let mut entries = entries_opt.unwrap();
                let count_usize = count as usize;
                let modified = modify_entries(&mut entries, count_usize, param2);
                if modified != 0 {
                    result = modified.wrapping_add(param3);
                } else {
                    result = modified;
                }
                drop(entries);
            }
        }
        3 => {
            if param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3 {
                let mut lookup_result: i32 = 0;
                let r = calculate_lookup(param1 as usize, param2 as usize, &mut lookup_result);
                if r != 0 {
                    result = lookup_result.wrapping_add(param3);
                } else {
                    result = r;
                }
            }
        }
        _ => {
            // strcpy(buffer, "Default")
            c_strcpy(&mut buffer, b"Default\0");
            result = process_name(&mut buffer, b"TestName\0", NAME_LENGTH);
            let count = c_strlen(&buffer) as i32;
            if count != 0 {
                result = count.wrapping_mul(param1);
            }
        }
    }

    result
}

// Read all of stdin and parse whitespace-separated integers, the way
// `scanf("%d", ...)` does (skipping any whitespace, including newlines).
fn read_ints_from_stdin() -> Vec<i32> {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);
    input
        .split_ascii_whitespace()
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}

fn main() {
    let nums = read_ints_from_stdin();
    let a = *nums.first().unwrap_or(&0);
    let b = *nums.get(1).unwrap_or(&0);
    let c = *nums.get(2).unwrap_or(&0);
    let d = *nums.get(3).unwrap_or(&0);
    let result = dataentry(a, b, c, d);
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let _ = writeln!(handle, "{}", result);
}
