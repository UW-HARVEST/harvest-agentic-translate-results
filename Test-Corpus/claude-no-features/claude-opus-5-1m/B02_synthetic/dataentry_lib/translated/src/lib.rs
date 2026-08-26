// Copyright 2025 MIT Lincoln Laboratory
// Rust translation preserving exact C behavior.

use std::ffi::c_int;

const NAME_LENGTH: usize = 32;

#[repr(C)]
#[derive(Copy, Clone)]
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

/// Linear search through the entries array.
unsafe fn find_entry(
    entries: *mut DataEntry,
    count: c_int,
    target_id: c_int,
) -> *mut DataEntry {
    let mut ptr = entries;
    let end = unsafe { entries.add(count as usize) };

    while ptr < end {
        if unsafe { (*ptr).id } == target_id {
            return ptr;
        }
        ptr = unsafe { ptr.add(1) };
    }

    std::ptr::null_mut()
}

/// Compute strlen on a C-style nul-terminated buffer.
unsafe fn c_strlen(p: *const u8) -> usize {
    let mut len = 0usize;
    while unsafe { *p.add(len) } != 0 {
        len += 1;
    }
    len
}

/// Copy a nul-terminated C string from src to dest (including the terminator).
unsafe fn c_strcpy(dest: *mut u8, src: *const u8) {
    let mut i = 0usize;
    loop {
        let b = unsafe { *src.add(i) };
        unsafe { *dest.add(i) = b };
        if b == 0 {
            break;
        }
        i += 1;
    }
}

/// Mirrors the C process_name. Returns -1 if dest is empty/NULL, otherwise
/// copies src to dest and returns the new length.
unsafe fn process_name(dest: *mut u8, src: *const u8, _max_len: c_int) -> c_int {
    if dest.is_null() || unsafe { *dest } == 0 {
        return -1;
    }

    unsafe { c_strcpy(dest, src) };

    unsafe { c_strlen(dest) as c_int }
}

fn calculate_lookup(row: c_int, col: c_int, result: &mut c_int) -> c_int {
    let temp = LOOKUP_TABLE[row as usize][col as usize];
    if temp != 0 {
        *result = temp * 2;
        return 1;
    }
    0
}

/// Mirrors the C create_entries: malloc'd buffer (in C). We allocate via
/// libc malloc to match free(). To stay portable without linking libc
/// directly, we use Box::into_raw with Layout via std::alloc.
unsafe fn create_entries(count: c_int, base_id: c_int) -> *mut DataEntry {
    // Replicate the C behavior: malloc(count * sizeof(DataEntry)) is called
    // first, then `entries == NULL || count <= 0` is checked. In the C code
    // on negative/zero count the allocation may succeed or fail; we then
    // return NULL (leaking the allocation if it succeeded). Reproduce the
    // visible behavior: return NULL if count <= 0 (don't actually allocate
    // when we can't, since negative * sizeof would overflow size_t).
    let total_bytes = (count as isize).wrapping_mul(std::mem::size_of::<DataEntry>() as isize);

    let entries: *mut DataEntry = if total_bytes <= 0 {
        std::ptr::null_mut()
    } else {
        let layout = match std::alloc::Layout::from_size_align(
            total_bytes as usize,
            std::mem::align_of::<DataEntry>(),
        ) {
            Ok(l) => l,
            Err(_) => return std::ptr::null_mut(),
        };
        unsafe { std::alloc::alloc(layout) as *mut DataEntry }
    };

    if entries.is_null() || count <= 0 {
        return std::ptr::null_mut();
    }

    for i in 0..count {
        let entry = unsafe { entries.add(i as usize) };
        unsafe {
            (*entry).id = base_id + i;
            (*entry).value = (base_id + i) * 10;
        }

        // sprintf(temp_name, "Entry_%d", base_id + i);
        let temp_name = format!("Entry_{}", base_id + i);
        let bytes = temp_name.as_bytes();

        // strcpy(entries[i].name, temp_name) -- copy bytes plus terminator.
        unsafe {
            let dest = (*entry).name.as_mut_ptr();
            for (j, &b) in bytes.iter().enumerate() {
                *dest.add(j) = b;
            }
            *dest.add(bytes.len()) = 0;
        }
    }

    entries
}

unsafe fn free_entries(entries: *mut DataEntry, count: c_int) {
    if entries.is_null() {
        return;
    }
    let total_bytes = (count as usize).wrapping_mul(std::mem::size_of::<DataEntry>());
    if total_bytes == 0 {
        return;
    }
    let layout = std::alloc::Layout::from_size_align(
        total_bytes,
        std::mem::align_of::<DataEntry>(),
    )
    .unwrap();
    unsafe { std::alloc::dealloc(entries as *mut u8, layout) };
}

unsafe fn modify_entries(
    entries: *mut DataEntry,
    count: c_int,
    multiplier: c_int,
) -> c_int {
    if entries.is_null() {
        return -1;
    }

    let mut total: c_int = 0;
    let mut current = entries;
    let last = unsafe { entries.add(count as usize) };

    while current < last {
        let temp_value = unsafe { (*current).value };
        if temp_value != 0 {
            let new_value = temp_value.wrapping_mul(multiplier);
            unsafe { (*current).value = new_value };
            total = total.wrapping_add(new_value);
        }
        current = unsafe { current.add(1) };
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn dataentry(mode: c_int, param1: c_int, param2: c_int, param3: c_int) -> c_int {
    let mut entries: *mut DataEntry = std::ptr::null_mut();
    let mut result: c_int = 0;
    let count: c_int;
    let mut lookup_result: c_int = 0;
    let mut buffer: [u8; NAME_LENGTH] = [0; NAME_LENGTH];

    buffer[0] = b'T';
    buffer[1] = 0;

    match mode {
        1 => {
            count = if param1 > 0 { param1 } else { 5 };
            entries = unsafe { create_entries(count, 100) };

            if entries.is_null() || count == 0 {
                result = -1;
            } else {
                let found = unsafe { find_entry(entries, count, 100 + param2) };

                if found.is_null() || unsafe { (*found).id } == 0 {
                    result = -2;
                } else {
                    result = unsafe { (*found).value };

                    // strcpy(buffer, found->name)
                    unsafe { c_strcpy(buffer.as_mut_ptr(), (*found).name.as_ptr()) };
                }

                unsafe { free_entries(entries, count) };
            }
        }
        2 => {
            count = if param1 > 0 { param1 } else { 3 };
            entries = unsafe { create_entries(count, 200) };

            if entries.is_null() {
                result = -1;
            } else {
                result = unsafe { modify_entries(entries, count, param2) };
                if result != 0 {
                    result = result.wrapping_add(param3);
                }

                unsafe { free_entries(entries, count) };
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
            // strcpy(buffer, "Default")
            let default_str = b"Default\0";
            for (i, &b) in default_str.iter().enumerate() {
                buffer[i] = b;
            }

            let test_name = b"TestName\0";
            result = unsafe {
                process_name(
                    buffer.as_mut_ptr(),
                    test_name.as_ptr(),
                    NAME_LENGTH as c_int,
                )
            };

            let count_local = unsafe { c_strlen(buffer.as_ptr()) as c_int };
            if count_local != 0 {
                result = count_local.wrapping_mul(param1);
            }
        }
    }

    let _ = entries; // silence unused warning where appropriate
    result
}
