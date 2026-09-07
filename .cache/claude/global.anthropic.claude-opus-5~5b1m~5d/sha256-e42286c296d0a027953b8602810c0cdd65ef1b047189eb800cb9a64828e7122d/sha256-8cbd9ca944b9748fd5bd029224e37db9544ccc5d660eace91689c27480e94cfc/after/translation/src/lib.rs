// Rust translation of c_src/src/lib.c (public header: c_src/include/lib.h)
//
// Original C copyright header preserved for reference:
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
// The translation is intentionally literal: every quirk of the C code
// (post-allocation NULL/count checks, the unused `max_len` parameter of
// `process_name`, dead `id == 0` / `count == 0` tests, wrapping signed
// arithmetic as produced by the reference compiler) is reproduced as-is.

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

// #define MAX_ENTRIES 10
const MAX_ENTRIES: c_int = 10;
// #define NAME_LENGTH 32
const NAME_LENGTH: usize = 32;

// typedef struct { int id; int value; char name[NAME_LENGTH]; } DataEntry;
#[repr(C)]
#[derive(Copy, Clone)]
struct DataEntry {
    id: c_int,
    value: c_int,
    name: [c_char; NAME_LENGTH],
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

// static int lookup_table[4][3] = {...};
static LOOKUP_TABLE: [[c_int; 3]; 4] = [
    [10, 20, 30],
    [40, 50, 60],
    [70, 80, 90],
    [100, 110, 120],
];

// ---------------------------------------------------------------------------
// Minimal, faithful re-implementations of the libc string routines used by the
// C source. They operate on raw C strings exactly like strlen()/strcpy().
// ---------------------------------------------------------------------------

unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

unsafe fn c_strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut i: usize = 0;
    unsafe {
        loop {
            let ch = *src.add(i);
            *dest.add(i) = ch;
            if ch == 0 {
                break;
            }
            i += 1;
        }
    }
    dest
}

/// Copy a Rust byte string (without its trailing NUL) plus a terminating NUL
/// into `dest`, mirroring `strcpy(dest, "literal")`.
unsafe fn c_strcpy_lit(dest: *mut c_char, src: &[u8]) {
    unsafe {
        let mut i: usize = 0;
        while i < src.len() {
            *dest.add(i) = src[i] as c_char;
            i += 1;
        }
        *dest.add(src.len()) = 0;
    }
}

/// `sprintf(dest, "Entry_%d", value)` — %d formats an int exactly the way
/// Rust formats an i32, so the resulting bytes are identical.
unsafe fn sprintf_entry_name(dest: *mut c_char, value: c_int) {
    // "Entry_" + at most 11 chars ("-2147483648") + NUL == 18 bytes.
    let mut tmp = [0u8; 24];
    let mut len: usize = 0;

    for &b in b"Entry_" {
        tmp[len] = b;
        len += 1;
    }

    // Decimal rendering of `value` (two's-complement safe for i32::MIN).
    let mut digits = [0u8; 12];
    let mut ndigits: usize = 0;
    let negative = value < 0;
    let mut mag: u32 = if negative {
        (value as i64).unsigned_abs() as u32
    } else {
        value as u32
    };
    if mag == 0 {
        digits[0] = b'0';
        ndigits = 1;
    } else {
        while mag > 0 {
            digits[ndigits] = b'0' + (mag % 10) as u8;
            mag /= 10;
            ndigits += 1;
        }
    }
    if negative {
        tmp[len] = b'-';
        len += 1;
    }
    while ndigits > 0 {
        ndigits -= 1;
        tmp[len] = digits[ndigits];
        len += 1;
    }

    unsafe {
        c_strcpy_lit(dest, &tmp[..len]);
    }
}

// ---------------------------------------------------------------------------
// static DataEntry* find_entry(DataEntry* entries, int count, int target_id)
// ---------------------------------------------------------------------------
unsafe fn find_entry(entries: *mut DataEntry, count: c_int, target_id: c_int) -> *mut DataEntry {
    let mut p: *mut DataEntry = entries;
    let end: *mut DataEntry = unsafe { entries.offset(count as isize) };

    unsafe {
        while p < end {
            if (*p).id == target_id {
                return p;
            }
            p = p.add(1);
        }
    }

    ptr::null_mut()
}

// ---------------------------------------------------------------------------
// static int process_name(char* dest, const char* src, int max_len)
// NOTE: `max_len` is never used by the C code and `strcpy` is unbounded; the
// odd `*dest == '\0'` guard is preserved verbatim.
// ---------------------------------------------------------------------------
unsafe fn process_name(dest: *mut c_char, src: *const c_char, max_len: c_int) -> c_int {
    let len: c_int;

    unsafe {
        if dest.is_null() || *dest == 0 {
            return -1;
        }

        c_strcpy(dest, src);

        len = c_strlen(dest) as c_int;
    }
    len
}

/// Variant used when the C code passes a string literal as `src`.
unsafe fn process_name_lit(dest: *mut c_char, src: &[u8], max_len: c_int) -> c_int {
    let len: c_int;

    unsafe {
        if dest.is_null() || *dest == 0 {
            return -1;
        }

        c_strcpy_lit(dest, src);

        len = c_strlen(dest) as c_int;
    }
    len
}

// ---------------------------------------------------------------------------
// static int calculate_lookup(int row, int col, int* result)
// ---------------------------------------------------------------------------
unsafe fn calculate_lookup(row: c_int, col: c_int, result: *mut c_int) -> c_int {
    let temp: c_int = LOOKUP_TABLE[row as usize][col as usize];

    if temp != 0 {
        unsafe {
            *result = temp.wrapping_mul(2);
        }
        return 1;
    }

    0
}

// ---------------------------------------------------------------------------
// static DataEntry* create_entries(int count, int base_id)
// NOTE: malloc() happens *before* the `count <= 0` check, exactly as in C
// (the allocation is leaked on the `count <= 0` path). `count * sizeof(...)`
// is computed with C's usual conversions: the int operand is converted to
// size_t (sign extension for negative counts).
// ---------------------------------------------------------------------------
unsafe fn create_entries(count: c_int, base_id: c_int) -> *mut DataEntry {
    let entries: *mut DataEntry;
    let mut i: c_int;
    let mut temp_name = [0 as c_char; NAME_LENGTH];

    let size = (count as isize as usize).wrapping_mul(core::mem::size_of::<DataEntry>());
    entries = unsafe { malloc(size) } as *mut DataEntry;

    if entries.is_null() || count <= 0 {
        return ptr::null_mut();
    }

    i = 0;
    while i < count {
        unsafe {
            let e = entries.offset(i as isize);
            (*e).id = base_id.wrapping_add(i);
            (*e).value = base_id.wrapping_add(i).wrapping_mul(10);

            sprintf_entry_name(temp_name.as_mut_ptr(), base_id.wrapping_add(i));

            c_strcpy((*e).name.as_mut_ptr(), temp_name.as_ptr());
        }
        i += 1;
    }

    entries
}

// ---------------------------------------------------------------------------
// static int modify_entries(DataEntry* entries, int count, int multiplier)
// ---------------------------------------------------------------------------
unsafe fn modify_entries(entries: *mut DataEntry, count: c_int, multiplier: c_int) -> c_int {
    let mut current: *mut DataEntry;
    let last: *mut DataEntry;
    let mut total: c_int = 0;
    let temp_value: c_int;

    if entries.is_null() {
        return -1;
    }

    current = entries;
    last = unsafe { entries.offset(count as isize) };

    unsafe {
        while current < last {
            let temp_value = (*current).value;
            if temp_value != 0 {
                (*current).value = temp_value.wrapping_mul(multiplier);
                total = total.wrapping_add((*current).value);
            }
            current = current.add(1);
        }
    }

    total
}

// ---------------------------------------------------------------------------
// int dataentry(int mode, int param1, int param2, int param3)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dataentry(
    mode: c_int,
    param1: c_int,
    param2: c_int,
    param3: c_int,
) -> c_int {
    let mut entries: *mut DataEntry = ptr::null_mut();
    let mut found: *mut DataEntry = ptr::null_mut();
    let mut result: c_int = 0;
    let count: c_int;
    let mut lookup_result: c_int = 0;
    let mut buffer = [0 as c_char; NAME_LENGTH];
    let _i: c_int;

    buffer[0] = b'T' as c_char;
    buffer[1] = 0;

    unsafe {
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

                        c_strcpy(buffer.as_mut_ptr(), (*found).name.as_ptr());
                    }

                    free(entries as *mut c_void);
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

                    free(entries as *mut c_void);
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
                c_strcpy_lit(buffer.as_mut_ptr(), b"Default");
                result = process_name_lit(buffer.as_mut_ptr(), b"TestName", NAME_LENGTH as c_int);

                count = c_strlen(buffer.as_ptr()) as c_int;
                if count != 0 {
                    result = count.wrapping_mul(param1);
                }
            }
        }
    }

    result
}
