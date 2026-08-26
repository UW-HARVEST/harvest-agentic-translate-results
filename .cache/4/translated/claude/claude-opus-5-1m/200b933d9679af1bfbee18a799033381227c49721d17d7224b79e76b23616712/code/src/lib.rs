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
// Faithful Rust translation of c_src/src/lib.c.
// Public ABI: `int dataentry(int, int, int, int)`.

use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_int;
use std::ptr;

const NAME_LENGTH: usize = 32;

/// Mirrors the C `DataEntry` struct:
/// ```c
/// typedef struct {
///     int id;
///     int value;
///     char name[NAME_LENGTH];
/// } DataEntry;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [u8; NAME_LENGTH],
}

/// `static int lookup_table[4][3]`
static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

// ---------------------------------------------------------------------------
// Small C runtime helpers (malloc/free/strcpy/strlen/sprintf semantics)
// ---------------------------------------------------------------------------

/// `malloc(nbytes)` for `DataEntry` storage.
///
/// Returns a null pointer whenever the underlying allocator would fail, which
/// is exactly the condition the C code checks for.
fn c_malloc_entries(nbytes: usize) -> *mut DataEntry {
    if nbytes == 0 {
        // malloc(0) returns a unique, non-null pointer. The C code never
        // dereferences or frees it (it bails out on `count <= 0` right after),
        // so a dangling but suitably aligned pointer reproduces the behavior.
        return ptr::NonNull::<DataEntry>::dangling().as_ptr();
    }

    if nbytes > isize::MAX as usize {
        return ptr::null_mut();
    }

    let layout = match Layout::from_size_align(nbytes, std::mem::align_of::<DataEntry>()) {
        Ok(layout) => layout,
        Err(_) => return ptr::null_mut(),
    };

    // SAFETY: `layout` has a non-zero size.
    unsafe { alloc(layout) as *mut DataEntry }
}

/// `free(entries)` where `entries` was allocated for `count` elements.
unsafe fn c_free_entries(entries: *mut DataEntry, count: c_int) {
    if entries.is_null() {
        return;
    }

    let nbytes = (count as isize as usize).wrapping_mul(std::mem::size_of::<DataEntry>());
    if nbytes == 0 || nbytes > isize::MAX as usize {
        return;
    }

    if let Ok(layout) = Layout::from_size_align(nbytes, std::mem::align_of::<DataEntry>()) {
        dealloc(entries as *mut u8, layout);
    }
}

/// `strlen(s)` over a NUL-terminated byte buffer.
fn c_strlen(buf: &[u8]) -> usize {
    buf.iter().position(|&b| b == 0).unwrap_or(buf.len())
}

/// `strcpy(dest, src)` where `src` is a Rust byte string (without NUL).
fn c_strcpy(dest: &mut [u8], src: &[u8]) {
    let n = src.len();
    dest[..n].copy_from_slice(src);
    dest[n] = 0;
}

/// `strcpy(dest, src)` where `src` is a NUL-terminated byte buffer.
fn c_strcpy_from_buf(dest: &mut [u8], src: &[u8]) {
    let n = c_strlen(src);
    dest[..n].copy_from_slice(&src[..n]);
    dest[n] = 0;
}

/// `sprintf(dest, "Entry_%d", value)`
fn sprintf_entry_name(dest: &mut [u8; NAME_LENGTH], value: c_int) {
    let mut digits = [0u8; 16];
    let mut len = 0usize;

    // Render the decimal representation the way printf("%d") does, handling
    // INT_MIN without overflowing.
    let negative = value < 0;
    let mut magnitude = (value as i64).unsigned_abs();
    if magnitude == 0 {
        digits[len] = b'0';
        len += 1;
    } else {
        while magnitude > 0 {
            digits[len] = b'0' + (magnitude % 10) as u8;
            magnitude /= 10;
            len += 1;
        }
    }

    let mut out = [0u8; NAME_LENGTH];
    let mut pos = 0usize;
    for &b in b"Entry_" {
        out[pos] = b;
        pos += 1;
    }
    if negative {
        out[pos] = b'-';
        pos += 1;
    }
    for i in (0..len).rev() {
        out[pos] = digits[i];
        pos += 1;
    }
    out[pos] = 0;

    *dest = out;
}

// ---------------------------------------------------------------------------
// Translated static helpers
// ---------------------------------------------------------------------------

/// ```c
/// static DataEntry* find_entry(DataEntry* entries, int count, int target_id);
/// ```
unsafe fn find_entry(entries: *mut DataEntry, count: c_int, target_id: c_int) -> *mut DataEntry {
    let mut ptr_cur = entries;
    let end = entries.offset(count as isize);

    while ptr_cur < end {
        if (*ptr_cur).id == target_id {
            return ptr_cur;
        }
        ptr_cur = ptr_cur.add(1);
    }

    ptr::null_mut()
}

/// ```c
/// static int process_name(char* dest, const char* src, int max_len);
/// ```
///
/// Note: `max_len` is unused by the C implementation (the copy is unbounded);
/// that behavior is preserved verbatim.
fn process_name(dest: &mut [u8], src: &[u8], _max_len: c_int) -> c_int {
    // `dest == NULL` cannot happen for the single (stack buffer) call site;
    // the `*dest == '\0'` check is preserved.
    if dest.is_empty() || dest[0] == 0 {
        return -1;
    }

    c_strcpy(dest, src);

    let len = c_strlen(dest);
    len as c_int
}

/// ```c
/// static int calculate_lookup(int row, int col, int* result);
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
/// static DataEntry* create_entries(int count, int base_id);
/// ```
unsafe fn create_entries(count: c_int, base_id: c_int) -> *mut DataEntry {
    let mut temp_name = [0u8; NAME_LENGTH];

    // malloc(count * sizeof(DataEntry)) -- `count` is converted to size_t,
    // exactly as C does (so negative counts wrap into a huge request).
    let nbytes = (count as isize as usize).wrapping_mul(std::mem::size_of::<DataEntry>());
    let entries = c_malloc_entries(nbytes);

    if entries.is_null() || count <= 0 {
        return ptr::null_mut();
    }

    let mut i: c_int = 0;
    while i < count {
        let entry = entries.offset(i as isize);
        (*entry).id = base_id.wrapping_add(i);
        (*entry).value = base_id.wrapping_add(i).wrapping_mul(10);

        sprintf_entry_name(&mut temp_name, base_id.wrapping_add(i));

        let mut name = [0u8; NAME_LENGTH];
        c_strcpy_from_buf(&mut name, &temp_name);
        (*entry).name = name;

        i += 1;
    }

    entries
}

/// ```c
/// static int modify_entries(DataEntry* entries, int count, int multiplier);
/// ```
unsafe fn modify_entries(entries: *mut DataEntry, count: c_int, multiplier: c_int) -> c_int {
    let mut total: c_int = 0;

    if entries.is_null() {
        return -1;
    }

    let mut current = entries;
    let last = entries.offset(count as isize);

    while current < last {
        let temp_value = (*current).value;
        if temp_value != 0 {
            (*current).value = temp_value.wrapping_mul(multiplier);
            total = total.wrapping_add((*current).value);
        }
        current = current.add(1);
    }

    total
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// ```c
/// int dataentry(int mode, int param1, int param2, int param3);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    unsafe {
        let mut entries: *mut DataEntry = ptr::null_mut();
        let found: *mut DataEntry;
        let mut result: c_int = 0;
        let count: c_int;
        let mut lookup_result: c_int = 0;
        let mut buffer = [0u8; NAME_LENGTH];

        buffer[0] = b'T';
        buffer[1] = 0;

        match mode {
            1 => {
                count = if param1 > 0 { param1 } else { 5 };
                entries = create_entries(count, 100);

                if entries.is_null() || count == 0 {
                    result = -1;
                } else {
                    found = find_entry(entries, count, 100i32.wrapping_add(param2));

                    if found.is_null() || (*found).id == 0 {
                        result = -2;
                    } else {
                        result = (*found).value;

                        let name = (*found).name;
                        c_strcpy_from_buf(&mut buffer, &name);
                    }

                    c_free_entries(entries, count);
                }
            }

            2 => {
                count = if param1 > 0 { param1 } else { 3 };
                entries = create_entries(count, 200);

                if entries.is_null() {
                    result = -1;
                } else {
                    result = modify_entries(entries, count, param2);
                    if result != 0 {
                        result = result.wrapping_add(param3);
                    }

                    c_free_entries(entries, count);
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
                c_strcpy(&mut buffer, b"Default");
                result = process_name(&mut buffer, b"TestName", NAME_LENGTH as c_int);

                let strlen_count = c_strlen(&buffer) as c_int;
                if strlen_count != 0 {
                    result = strlen_count.wrapping_mul(param1);
                }
            }
        }

        // Silence "assigned but never read" style warnings while keeping the
        // same locals as the C source.
        let _ = entries;

        result
    }
}
