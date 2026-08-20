/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
//! generic_containers.rs
//!
//! Translation of the `DECLARE_ARRAY`/`DEFINE_ARRAY` and
//! `DECLARE_LIST`/`DEFINE_LIST` macro families in `generic_containers.h`.
//!
//! The C code monomorphizes one copy per element type through the preprocessor;
//! Rust generics do the same job with one body. The struct layouts are
//! `#[repr(C)]` and the storage comes from the same `malloc`/`realloc`/`free`,
//! so a container built by the C library and one built here are interchangeable
//! down to the byte.

#![allow(dead_code)]

use std::mem::size_of;
use std::ptr;

use libc::{c_int, c_void};

// ============================================================================
// GENERIC DYNAMIC ARRAY
// ============================================================================

/// `array_TYPE_t`
#[repr(C)]
pub struct ArrayT<T> {
    pub data: *mut T,
    pub size: usize,
    pub capacity: usize,
}

/// `array_TYPE_create(initial_capacity)`
///
/// # Safety
/// Raw-pointer API mirroring the C function; the result must eventually be
/// released with [`array_destroy`].
pub unsafe fn array_create<T>(initial_capacity: usize) -> *mut ArrayT<T> {
    let arr = libc::malloc(size_of::<ArrayT<T>>()) as *mut ArrayT<T>;
    if arr.is_null() {
        return ptr::null_mut();
    }
    (*arr).capacity = if initial_capacity > 0 {
        initial_capacity
    } else {
        16
    };
    (*arr).size = 0;
    // `sizeof(TYPE) * arr->capacity` wraps around in C rather than trapping.
    (*arr).data = libc::malloc(size_of::<T>().wrapping_mul((*arr).capacity)) as *mut T;
    if (*arr).data.is_null() {
        libc::free(arr as *mut c_void);
        return ptr::null_mut();
    }
    arr
}

/// `array_TYPE_destroy(arr)`
///
/// # Safety
/// `arr` must be NULL or a pointer previously returned by [`array_create`].
pub unsafe fn array_destroy<T>(arr: *mut ArrayT<T>) {
    if !arr.is_null() {
        libc::free((*arr).data as *mut c_void);
        libc::free(arr as *mut c_void);
    }
}

/// `array_TYPE_push(arr, value)`
///
/// # Safety
/// `arr` must be NULL or a valid array pointer.
pub unsafe fn array_push<T>(arr: *mut ArrayT<T>, value: T) -> c_int {
    if arr.is_null() {
        return -1;
    }
    if (*arr).size >= (*arr).capacity {
        let new_capacity = (*arr).capacity.wrapping_mul(2);
        let new_data = libc::realloc(
            (*arr).data as *mut c_void,
            size_of::<T>().wrapping_mul(new_capacity),
        ) as *mut T;
        if new_data.is_null() {
            return -1;
        }
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    // arr->data[arr->size++] = value;
    ptr::write((*arr).data.add((*arr).size), value);
    (*arr).size += 1;
    0
}

/// `array_TYPE_get(arr, index)` -- like C, neither `arr` nor `index` is checked.
///
/// The two loads go through `read_volatile` so that a NULL `arr` faults exactly
/// the way the C code does (a plain Rust dereference would instead trip the
/// compiler's debug-only null-pointer assertion and abort).
///
/// # Safety
/// `arr` must be a valid array pointer and `index` in bounds, exactly as the C
/// function requires.
pub unsafe fn array_get<T: Copy>(arr: *mut ArrayT<T>, index: usize) -> T {
    let data = ptr::read_volatile(ptr::addr_of!((*arr).data));
    ptr::read_volatile(data.add(index))
}

/// `array_TYPE_size(arr)`
///
/// # Safety
/// `arr` must be NULL or a valid array pointer.
pub unsafe fn array_size<T>(arr: *mut ArrayT<T>) -> usize {
    if arr.is_null() {
        0
    } else {
        (*arr).size
    }
}

/// `array_TYPE_clear(arr)`
///
/// # Safety
/// `arr` must be NULL or a valid array pointer.
pub unsafe fn array_clear<T>(arr: *mut ArrayT<T>) {
    if !arr.is_null() {
        (*arr).size = 0;
    }
}

/// `ARRAY_FOREACH(TYPE, var, arr)`: re-reads `arr->size` on every iteration and
/// copies each element into the loop variable, just like the macro.
///
/// # Safety
/// `arr` must be a valid, non-NULL array pointer (the macro dereferences it
/// unconditionally).
pub unsafe fn array_foreach<T: Copy, F: FnMut(T)>(arr: *mut ArrayT<T>, mut body: F) {
    let mut i: usize = 0;
    while i < (*arr).size {
        let var = *(*arr).data.add(i);
        body(var);
        i += 1;
    }
}

// ============================================================================
// GENERIC LINKED LIST
// ============================================================================

/// `list_node_TYPE_t`
#[repr(C)]
pub struct ListNodeT<T> {
    pub data: T,
    pub next: *mut ListNodeT<T>,
}

/// `list_TYPE_t`
#[repr(C)]
pub struct ListT<T> {
    pub head: *mut ListNodeT<T>,
    pub tail: *mut ListNodeT<T>,
    pub size: usize,
}

/// `list_TYPE_create()`
///
/// # Safety
/// Raw-pointer API mirroring the C function.
pub unsafe fn list_create<T>() -> *mut ListT<T> {
    let list = libc::malloc(size_of::<ListT<T>>()) as *mut ListT<T>;
    if list.is_null() {
        return ptr::null_mut();
    }
    (*list).head = ptr::null_mut();
    (*list).tail = ptr::null_mut();
    (*list).size = 0;
    list
}

/// `list_TYPE_destroy(list)`
///
/// # Safety
/// `list` must be NULL or a pointer previously returned by [`list_create`].
pub unsafe fn list_destroy<T>(list: *mut ListT<T>) {
    if list.is_null() {
        return;
    }
    let mut current = (*list).head;
    while !current.is_null() {
        let next = (*current).next;
        libc::free(current as *mut c_void);
        current = next;
    }
    libc::free(list as *mut c_void);
}

/// `list_TYPE_append(list, value)`
///
/// # Safety
/// `list` must be NULL or a valid list pointer.
pub unsafe fn list_append<T>(list: *mut ListT<T>, value: T) -> c_int {
    if list.is_null() {
        return -1;
    }
    let node = libc::malloc(size_of::<ListNodeT<T>>()) as *mut ListNodeT<T>;
    if node.is_null() {
        return -1;
    }
    ptr::write(ptr::addr_of_mut!((*node).data), value);
    (*node).next = ptr::null_mut();
    if (*list).head.is_null() {
        (*list).head = node;
        (*list).tail = node;
    } else {
        (*(*list).tail).next = node;
        (*list).tail = node;
    }
    (*list).size += 1;
    0
}

/// `list_TYPE_prepend(list, value)`
///
/// # Safety
/// `list` must be NULL or a valid list pointer.
pub unsafe fn list_prepend<T>(list: *mut ListT<T>, value: T) -> c_int {
    if list.is_null() {
        return -1;
    }
    let node = libc::malloc(size_of::<ListNodeT<T>>()) as *mut ListNodeT<T>;
    if node.is_null() {
        return -1;
    }
    ptr::write(ptr::addr_of_mut!((*node).data), value);
    (*node).next = (*list).head;
    (*list).head = node;
    if (*list).tail.is_null() {
        (*list).tail = node;
    }
    (*list).size += 1;
    0
}

/// `list_TYPE_size(list)`
///
/// # Safety
/// `list` must be NULL or a valid list pointer.
pub unsafe fn list_size<T>(list: *mut ListT<T>) -> usize {
    if list.is_null() {
        0
    } else {
        (*list).size
    }
}

/// `list_TYPE_clear(list)`
///
/// # Safety
/// `list` must be NULL or a valid list pointer.
pub unsafe fn list_clear<T>(list: *mut ListT<T>) {
    if list.is_null() {
        return;
    }
    let mut current = (*list).head;
    while !current.is_null() {
        let next = (*current).next;
        libc::free(current as *mut c_void);
        current = next;
    }
    (*list).head = ptr::null_mut();
    (*list).tail = ptr::null_mut();
    (*list).size = 0;
}

/// `LIST_FOREACH(TYPE, var, list)`
///
/// # Safety
/// `list` must be a valid, non-NULL list pointer (the macro dereferences it
/// unconditionally).
pub unsafe fn list_foreach<T: Copy, F: FnMut(T)>(list: *mut ListT<T>, mut body: F) {
    let mut node = (*list).head;
    while !node.is_null() {
        let var = (*node).data;
        body(var);
        node = (*node).next;
    }
}
