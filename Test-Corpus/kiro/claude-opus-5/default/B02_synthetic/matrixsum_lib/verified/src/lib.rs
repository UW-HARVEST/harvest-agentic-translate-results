// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
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
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Rust translation of c_src/src/lib.c. Behaviour, including the exact order of
// checks and the integer arithmetic, mirrors the original C.

#![allow(non_upper_case_globals)]

use std::alloc::{Layout, alloc, dealloc, realloc};
use std::ffi::c_int;

/// `int matrix[3][4]` — an exported, mutable global in the original C.
#[unsafe(no_mangle)]
pub static mut matrix: [[c_int; 4]; 3] = [
    [0x01, 0x02, 0x03, 0x04],
    [0x10, 0x20, 0x30, 0x40],
    [0xA1, 0xB2, 0xC3, 0xD4],
];

const FLAG_READ: c_int = 0b0000_0001;
const FLAG_WRITE: c_int = 0b0000_0010;
const FLAG_EXECUTE: c_int = 0b0000_0100;
const FLAG_DELETE: c_int = 0b0000_1000;

/// Same layout as the C `DynamicArray` struct.
#[repr(C)]
pub struct DynamicArray {
    pub data: *mut c_int,
    pub size: usize,
    pub capacity: usize,
}

const INT_SIZE: usize = std::mem::size_of::<c_int>();
const INT_ALIGN: usize = std::mem::align_of::<c_int>();

/// Byte size of `n` ints, wrapping exactly like the C multiplication would.
fn int_bytes(n: usize) -> usize {
    n.wrapping_mul(INT_SIZE)
}

/// `bytes` comes from a wrapping multiply, so it can exceed what `Layout`
/// accepts (`isize::MAX` after rounding up to the alignment). A request that
/// large can never be serviced, which is exactly what `malloc`/`realloc` report
/// by returning NULL, so `None` is treated as an allocation failure rather than
/// a panic.
fn int_layout(bytes: usize) -> Option<Layout> {
    Layout::from_size_align(bytes, INT_ALIGN).ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn init_array(initial_capacity: usize) -> *mut DynamicArray {
    let arr = unsafe { alloc(Layout::new::<DynamicArray>()) } as *mut DynamicArray;
    if arr.is_null() {
        return std::ptr::null_mut();
    }

    let bytes = int_bytes(initial_capacity);
    let data = if bytes == 0 {
        // malloc(0) yields a non-null pointer that must not be dereferenced.
        INT_ALIGN as *mut c_int
    } else {
        match int_layout(bytes) {
            Some(layout) => (unsafe { alloc(layout) }) as *mut c_int,
            None => std::ptr::null_mut(),
        }
    };

    if data.is_null() {
        unsafe { dealloc(arr as *mut u8, Layout::new::<DynamicArray>()) };
        return std::ptr::null_mut();
    }

    unsafe {
        (*arr).data = data;
        (*arr).size = 0;
        (*arr).capacity = initial_capacity;
    }
    arr
}

#[unsafe(no_mangle)]
pub extern "C" fn expand_array(arr: *mut DynamicArray) -> c_int {
    if arr.is_null() {
        return 0;
    }
    let arr = unsafe { &mut *arr };

    let new_capacity = arr.capacity.wrapping_mul(2);
    let old_bytes = int_bytes(arr.capacity);
    let new_bytes = int_bytes(new_capacity);

    let new_data = if new_bytes == 0 {
        // realloc(ptr, 0) frees and returns NULL.
        if old_bytes != 0 {
            if let Some(layout) = int_layout(old_bytes) {
                unsafe { dealloc(arr.data as *mut u8, layout) };
            }
        }
        std::ptr::null_mut()
    } else {
        match int_layout(new_bytes) {
            // A size `Layout` rejects can never be allocated; realloc would fail
            // and leave the existing block untouched, so just report failure.
            None => std::ptr::null_mut(),
            Some(new_layout) => match int_layout(old_bytes) {
                // `old_bytes == 0` means `data` is the placeholder standing in
                // for malloc(0), which was never handed out by this allocator
                // and so cannot be passed to `realloc`.
                Some(old_layout) if old_bytes != 0 => {
                    (unsafe { realloc(arr.data as *mut u8, old_layout, new_bytes) }) as *mut c_int
                }
                _ => (unsafe { alloc(new_layout) }) as *mut c_int,
            },
        }
    };

    if new_data.is_null() {
        return 0;
    }

    arr.data = new_data;
    arr.capacity = new_capacity;
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn add_element(arr: *mut DynamicArray, value: c_int) -> c_int {
    if arr.is_null() {
        return 0;
    }

    {
        let a = unsafe { &*arr };
        if a.size >= a.capacity {
            if expand_array(arr) == 0 {
                return 0;
            }
        }
    }

    let a = unsafe { &mut *arr };
    unsafe { *a.data.add(a.size) = value };
    a.size += 1;
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn free_array(arr: *mut DynamicArray) {
    if !arr.is_null() {
        let a = unsafe { &mut *arr };
        let bytes = int_bytes(a.capacity);
        if bytes != 0 && !a.data.is_null() {
            if let Some(layout) = int_layout(bytes) {
                unsafe { dealloc(a.data as *mut u8, layout) };
            }
        }
        unsafe { dealloc(arr as *mut u8, Layout::new::<DynamicArray>()) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_flags(flags: c_int) -> c_int {
    let has_read = flags & FLAG_READ;
    let read_enabled = (has_read != 0) as c_int;

    let has_write = flags & FLAG_WRITE;
    let write_enabled = (has_write != 0) as c_int;

    let has_execute = flags & FLAG_EXECUTE;
    let execute_enabled = (has_execute != 0) as c_int;

    let has_delete = flags & FLAG_DELETE;
    let delete_enabled = (has_delete != 0) as c_int;

    read_enabled
        .wrapping_add(write_enabled)
        .wrapping_add(execute_enabled)
        .wrapping_add(delete_enabled)
}

#[unsafe(no_mangle)]
pub extern "C" fn calculate_matrix_checksum() -> c_int {
    let mut sum: c_int = 0;

    for i in 0..3 {
        for j in 0..4 {
            sum = sum.wrapping_add(unsafe { matrix[i][j] });
        }
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn matrixsum(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let hex_base: c_int = 0xFF;
    let hex_multiplier: c_int = 0x10;

    let mut permissions: c_int = 0b0000;

    let valid1 = param1 != 0;
    let valid2 = param2 != 0;
    let valid3 = param3 != 0;
    let valid4 = param4 != 0;

    if valid1 {
        permissions |= FLAG_READ;
    }
    if valid2 {
        permissions |= FLAG_WRITE;
    }
    if valid3 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 {
        permissions |= FLAG_DELETE;
    }

    let arr = init_array(2);
    if arr.is_null() {
        return -1;
    }

    add_element(arr, param1);
    add_element(arr, param2);
    add_element(arr, param3);
    add_element(arr, param4);

    let mut sum: c_int = 0;
    {
        let a = unsafe { &*arr };
        for i in 0..a.size {
            sum = sum.wrapping_add(unsafe { *a.data.add(i) });
        }
    }

    let flag_count = process_flags(permissions);
    let matrix_sum = calculate_matrix_checksum();

    let result = sum
        .wrapping_mul(hex_multiplier)
        .wrapping_add(flag_count.wrapping_mul(hex_base))
        .wrapping_add(matrix_sum & 0xFFF);

    free_array(arr);

    result
}
