// Rust translation of c_src/src/lib.c
//
// Original copyright notice from the C source:
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::c_int;

const NAME_LENGTH: usize = 32;

/// Mirror of the C `DataEntry` struct. `name` is a fixed-size NUL-terminated
/// byte buffer, exactly like `char name[NAME_LENGTH]`.
#[derive(Clone, Copy)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

impl DataEntry {
    const fn zeroed() -> Self {
        DataEntry {
            id: 0,
            value: 0,
            name: [0u8; NAME_LENGTH],
        }
    }
}

/// `static int lookup_table[4][3]`
static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

/// Equivalent of C `strlen` over a fixed NUL-terminated buffer.
fn c_strlen(buf: &[u8]) -> usize {
    match buf.iter().position(|&b| b == 0) {
        Some(n) => n,
        // A buffer without a terminator would be UB in C; treat the whole
        // buffer as the string, which is the closest well-defined behaviour.
        None => buf.len(),
    }
}

/// Equivalent of C `strcpy(dest, src)` where `src` is a NUL-terminated view.
/// Copies the bytes up to and including the terminator.
fn c_strcpy(dest: &mut [u8], src: &[u8]) {
    let len = c_strlen(src);
    dest[..len].copy_from_slice(&src[..len]);
    dest[len] = 0;
}

/// `static DataEntry* find_entry(DataEntry*, int, int)`
///
/// Returns the index of the first entry whose id matches, or `None`.
/// Note that a negative `count` yields `end < entries`, so the loop body never
/// runs and `NULL` is returned - matching the C pointer comparison.
fn find_entry(entries: &[DataEntry], count: c_int, target_id: c_int) -> Option<usize> {
    if count <= 0 {
        return None;
    }
    let count = count as usize;
    for i in 0..count {
        if entries[i].id == target_id {
            return Some(i);
        }
    }
    None
}

/// `static int process_name(char* dest, const char* src, int max_len)`
///
/// `max_len` is accepted but unused, exactly as in the C original (the copy is
/// unbounded). The guard rejects an empty destination string, not a null one
/// only - reproduced verbatim.
fn process_name(dest: &mut [u8], src: &[u8], _max_len: c_int) -> c_int {
    // `dest == NULL` cannot happen for a Rust slice; the second half of the
    // original condition (`*dest == '\0'`) is what remains observable.
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }

    c_strcpy(dest, src);

    let len = c_strlen(dest);
    len as c_int
}

/// `static int calculate_lookup(int row, int col, int* result)`
fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];
    if temp != 0 {
        *result = temp.wrapping_mul(2);
        return 1;
    }

    0
}

/// `static DataEntry* create_entries(int count, int base_id)`
///
/// The C version allocates *before* validating `count`, and returns NULL when
/// the allocation fails or `count <= 0` (leaking the allocation in the latter
/// case). The observable result is `None` for any `count <= 0`, and `None` when
/// the allocation cannot be satisfied.
fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    // Emulate `malloc(count * sizeof(DataEntry))`: `count` is converted to
    // size_t, so a negative count becomes an enormous request that fails.
    let bytes = (count as usize).wrapping_mul(core::mem::size_of::<DataEntry>());
    let mut entries: Vec<DataEntry> = Vec::new();
    if entries
        .try_reserve_exact(bytes / core::mem::size_of::<DataEntry>())
        .is_err()
    {
        return None;
    }

    if count <= 0 {
        return None;
    }

    let count_usize = count as usize;
    entries.resize(count_usize, DataEntry::zeroed());

    for i in 0..count_usize {
        let idx = base_id.wrapping_add(i as c_int);
        entries[i].id = idx;
        entries[i].value = idx.wrapping_mul(10);

        // sprintf(temp_name, "Entry_%d", base_id + i) then strcpy into name.
        let temp_name = format!("Entry_{}\0", idx);
        c_strcpy(&mut entries[i].name, temp_name.as_bytes());
    }

    Some(entries)
}

/// `static int modify_entries(DataEntry*, int count, int multiplier)`
///
/// A NULL `entries` returns -1; that case is represented by the caller never
/// invoking this function with `None`.
fn modify_entries(entries: &mut [DataEntry], count: c_int, multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

    if count <= 0 {
        // `last < current`, so the loop never executes.
        return total;
    }

    for i in 0..(count as usize) {
        let temp_value = entries[i].value;
        if temp_value != 0 {
            entries[i].value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(entries[i].value);
        }
    }

    total
}

/// `int dataentry(int mode, int param1, int param2, int param3)`
#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: c_int = 0;
    let count: c_int;
    let mut lookup_result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            count = if param1 > 0 { param1 } else { 5 };
            let entries = create_entries(count, 100);

            let entries = match entries {
                None => return -1,
                Some(e) => e,
            };
            if count == 0 {
                return -1;
            }

            let found = find_entry(&entries, count, 100i32.wrapping_add(param2));

            match found {
                None => result = -2,
                Some(idx) => {
                    if entries[idx].id == 0 {
                        result = -2;
                    } else {
                        result = entries[idx].value;

                        let name = entries[idx].name;
                        c_strcpy(&mut buffer, &name);
                    }
                }
            }

            // free(entries): the Vec is dropped here.
            drop(entries);
        }

        2 => {
            count = if param1 > 0 { param1 } else { 3 };
            let entries = create_entries(count, 200);

            let mut entries = match entries {
                None => return -1,
                Some(e) => e,
            };

            result = modify_entries(&mut entries, count, param2);
            if result != 0 {
                result = result.wrapping_add(param3);
            }

            drop(entries);
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
            c_strcpy(&mut buffer, b"Default\0");
            result = process_name(&mut buffer, b"TestName\0", NAME_LENGTH as c_int);

            let len = c_strlen(&buffer) as c_int;
            if len != 0 {
                result = len.wrapping_mul(param1);
            }
        }
    }

    result
}
