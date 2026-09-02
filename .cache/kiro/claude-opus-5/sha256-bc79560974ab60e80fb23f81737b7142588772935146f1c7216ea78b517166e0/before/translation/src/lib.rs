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
//
// The C library exports exactly one public symbol: `dataentry`. Everything
// else in the translation unit is `static` and therefore has internal linkage.
// The helpers below are kept as private Rust functions with the same names and
// the same observable behavior, including the original quirks (dead checks,
// leaked allocation on the `count <= 0` path, unused `max_len` parameter,
// signed wraparound arithmetic).

use std::ffi::c_int;

/// `#define NAME_LENGTH 32`
const NAME_LENGTH: usize = 32;

/// `#define MAX_ENTRIES 10` — declared but never used by the C code.
#[allow(dead_code)]
const MAX_ENTRIES: usize = 10;

/// ```c
/// typedef struct {
///     int id;
///     int value;
///     char name[NAME_LENGTH];
/// } DataEntry;
/// ```
#[derive(Clone, Copy)]
#[repr(C)]
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

/// ```c
/// static int lookup_table[4][3] = {
///     {10, 20, 30},
///     {40, 50, 60},
///     {70, 80, 90},
///     {100, 110, 120}
/// };
/// ```
static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

/// Byte-for-byte equivalent of `strcpy(dest, src)` where `src` is a
/// NUL-terminated byte slice: copies bytes up to and including the terminator.
fn strcpy(dest: &mut [u8], src: &[u8]) {
    for (i, &b) in src.iter().enumerate() {
        dest[i] = b;
        if b == 0 {
            return;
        }
    }
}

/// `strcpy(dest, literal)` where the Rust literal carries no NUL of its own.
fn strcpy_str(dest: &mut [u8], src: &str) {
    let bytes = src.as_bytes();
    dest[..bytes.len()].copy_from_slice(bytes);
    dest[bytes.len()] = 0;
}

/// Equivalent of `strlen` over a NUL-terminated byte buffer.
fn strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// ```c
/// static DataEntry* find_entry(DataEntry* entries, int count, int target_id) {
///     DataEntry* ptr = entries;
///     DataEntry* end = entries + count;
///     while (ptr < end) {
///         if (ptr->id == target_id) { return ptr; }
///         ptr++;
///     }
///     return NULL;
/// }
/// ```
///
/// Returns the index of the match rather than a raw pointer. `count` is used
/// exactly as the C code uses it: as the pointer-arithmetic end bound. A
/// negative or oversized `count` would be out-of-bounds in C as well; here the
/// walk is clamped to the live slice, which is the same set of elements the C
/// loop visits for every `count` the public entry point can produce.
fn find_entry(entries: &[DataEntry], count: c_int, target_id: c_int) -> Option<usize> {
    if count <= 0 {
        // `ptr < end` is false immediately.
        return None;
    }
    let end = (count as usize).min(entries.len());
    for i in 0..end {
        if entries[i].id == target_id {
            return Some(i);
        }
    }
    None
}

/// ```c
/// static int process_name(char* dest, const char* src, int max_len) {
///     int len;
///     if (dest == NULL || *dest == '\0') { return -1; }
///     strcpy(dest, src);
///     len = strlen(dest);
///     return len;
/// }
/// ```
///
/// `max_len` is accepted and ignored, exactly as in the C original: the copy is
/// an unbounded `strcpy`. `dest` is a live buffer here, so the `dest == NULL`
/// half of the guard is unreachable, matching every call site in the C code.
fn process_name(dest: &mut [u8], src: &str, _max_len: c_int) -> c_int {
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }

    strcpy_str(dest, src);

    let len = strlen(dest);
    len as c_int
}

/// ```c
/// static int calculate_lookup(int row, int col, int* result) {
///     int temp;
///     if ((temp = lookup_table[row][col])) {
///         *result = temp * 2;
///         return 1;
///     }
///     return 0;
/// }
/// ```
fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];

    if temp != 0 {
        *result = temp.wrapping_mul(2);
        return 1;
    }

    0
}

/// ```c
/// static DataEntry* create_entries(int count, int base_id) {
///     DataEntry* entries;
///     int i;
///     char temp_name[NAME_LENGTH];
///
///     entries = (DataEntry*)malloc(count * sizeof(DataEntry));
///     if (entries == NULL || count <= 0) { return NULL; }
///
///     for (i = 0; i < count; i++) {
///         entries[i].id = base_id + i;
///         entries[i].value = (base_id + i) * 10;
///         sprintf(temp_name, "Entry_%d", base_id + i);
///         strcpy(entries[i].name, temp_name);
///     }
///     return entries;
/// }
/// ```
///
/// The C code allocates *before* validating `count`, so a non-positive `count`
/// leaks the block and still returns NULL. That ordering is preserved: the
/// allocation is attempted first and its failure is reported as NULL, then the
/// `count <= 0` check rejects the buffer (the Rust allocation is simply dropped
/// instead of leaked, which is not externally observable).
fn create_entries(count: c_int, base_id: c_int) -> Option<Vec<DataEntry>> {
    // `count * sizeof(DataEntry)`: `count` is converted to size_t, so a
    // negative count becomes an enormous request that malloc refuses.
    let bytes = if count < 0 {
        // Mirrors the wrap to a huge size_t; such a request cannot succeed.
        return None;
    } else {
        (count as usize).checked_mul(size_of::<DataEntry>())?
    };

    let mut entries: Vec<DataEntry> = Vec::new();
    if entries
        .try_reserve_exact(bytes / size_of::<DataEntry>())
        .is_err()
    {
        // malloc returned NULL.
        return None;
    }

    if count <= 0 {
        // Allocation succeeded but `count <= 0` still yields NULL.
        return None;
    }

    let mut temp_name = [0u8; NAME_LENGTH];

    entries.resize(count as usize, DataEntry::zeroed());

    for i in 0..count {
        let id = base_id.wrapping_add(i);

        entries[i as usize].id = id;
        entries[i as usize].value = id.wrapping_mul(10);

        // sprintf(temp_name, "Entry_%d", base_id + i)
        let formatted = format!("Entry_{}", id);
        strcpy_str(&mut temp_name, &formatted);

        let name = &mut entries[i as usize].name;
        strcpy(name, &temp_name);
    }

    Some(entries)
}

/// ```c
/// static int modify_entries(DataEntry* entries, int count, int multiplier) {
///     DataEntry* current;
///     DataEntry* last;
///     int total = 0;
///     int temp_value;
///
///     if (entries == NULL) { return -1; }
///
///     current = entries;
///     last = entries + count;
///     while (current < last) {
///         if ((temp_value = current->value)) {
///             current->value = temp_value * multiplier;
///             total += current->value;
///         }
///         current++;
///     }
///     return total;
/// }
/// ```
///
/// The `entries == NULL` branch is handled by the caller passing a live slice;
/// every call site in the C code checks for NULL beforehand.
fn modify_entries(entries: &mut [DataEntry], count: c_int, multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

    if count <= 0 {
        return total;
    }
    let last = (count as usize).min(entries.len());

    for i in 0..last {
        let temp_value = entries[i].value;
        if temp_value != 0 {
            entries[i].value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add(entries[i].value);
        }
    }

    total
}

/// ```c
/// int dataentry(int mode, int param1, int param2, int param3);
/// ```
///
/// Public entry point; the only symbol the C shared library exports.
#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut result: c_int = 0;
    let count: c_int;
    let mut lookup_result: c_int = 0;
    let mut buffer = [0u8; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = b'\0';

    match mode {
        1 => {
            count = if param1 > 0 { param1 } else { 5 };
            let entries = create_entries(count, 100);

            // if (entries == NULL || count == 0) { result = -1; break; }
            if entries.is_none() || count == 0 {
                return -1;
            }
            let entries = entries.unwrap();

            let found = find_entry(&entries, count, 100i32.wrapping_add(param2));

            // if (found == NULL || found->id == 0) { result = -2; }
            match found {
                None => {
                    result = -2;
                }
                Some(idx) if entries[idx].id == 0 => {
                    result = -2;
                }
                Some(idx) => {
                    result = entries[idx].value;

                    let name = entries[idx].name;
                    strcpy(&mut buffer, &name);
                }
            }

            // free(entries) — handled by dropping `entries`.
            drop(entries);
        }

        2 => {
            count = if param1 > 0 { param1 } else { 3 };
            let entries = create_entries(count, 200);

            if entries.is_none() {
                return -1;
            }
            let mut entries = entries.unwrap();

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
            strcpy_str(&mut buffer, "Default");
            result = process_name(&mut buffer, "TestName", NAME_LENGTH as c_int);

            let n = strlen(&buffer) as c_int;
            if n != 0 {
                result = n.wrapping_mul(param1);
            }
        }
    }

    // `buffer` is written but never read back out of the function, exactly as
    // in the C original.
    let _ = &buffer;

    result
}
