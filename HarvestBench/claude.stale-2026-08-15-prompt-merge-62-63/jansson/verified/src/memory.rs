//! Translation of memory.c
#![allow(non_upper_case_globals)]

use crate::types::*;
use core::ffi::{c_char, c_void};
use core::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

// memory function pointers, initialized to the libc functions (as in memory.c)
static mut do_malloc: json_malloc_t = Some(malloc);
static mut do_realloc: json_realloc_t = Some(realloc);
static mut do_free: json_free_t = Some(free);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }
    (do_malloc.unwrap())(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    (do_free.unwrap())(ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_realloc(
    ptr: *mut c_void,
    original_size: usize,
    new_size: usize,
) -> *mut c_void {
    if let Some(realloc_fn) = do_realloc {
        return realloc_fn(ptr, new_size);
    }

    // realloc emulation using malloc and free
    if new_size == 0 {
        if !ptr.is_null() {
            (do_free.unwrap())(ptr);
        }
        ptr::null_mut()
    } else {
        let new_memory = (do_malloc.unwrap())(new_size);

        if !new_memory.is_null() && !ptr.is_null() {
            memcpy(
                new_memory,
                ptr,
                if original_size < new_size {
                    original_size
                } else {
                    new_size
                },
            );
            (do_free.unwrap())(ptr);
        }

        new_memory
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strndup(str: *const c_char, len: usize) -> *mut c_char {
    // `wrapping_add`, not `+`: the C computes `size_t len + 1`, which wraps to 0
    // for len == (size_t)-1 and so reaches `jsonp_malloc(0)` -> NULL -> an early
    // NULL return. Rust's `+` would panic on that overflow under debug
    // overflow-checks, aborting instead of returning NULL like the C.
    let new_str = jsonp_malloc(len.wrapping_add(1)) as *mut c_char;
    if new_str.is_null() {
        return ptr::null_mut();
    }

    memcpy(new_str as *mut c_void, str as *const c_void, len);
    *new_str.add(len) = 0;
    new_str
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_set_alloc_funcs(malloc_fn: json_malloc_t, free_fn: json_free_t) {
    do_malloc = malloc_fn;
    do_realloc = None;
    do_free = free_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_set_alloc_funcs2(
    malloc_fn: json_malloc_t,
    realloc_fn: json_realloc_t,
    free_fn: json_free_t,
) {
    do_malloc = malloc_fn;
    do_realloc = realloc_fn;
    do_free = free_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs(
    malloc_fn: *mut json_malloc_t,
    free_fn: *mut json_free_t,
) {
    if !malloc_fn.is_null() {
        *malloc_fn = do_malloc;
    }
    if !free_fn.is_null() {
        *free_fn = do_free;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs2(
    malloc_fn: *mut json_malloc_t,
    realloc_fn: *mut json_realloc_t,
    free_fn: *mut json_free_t,
) {
    if !malloc_fn.is_null() {
        *malloc_fn = do_malloc;
    }
    if !realloc_fn.is_null() {
        *realloc_fn = do_realloc;
    }
    if !free_fn.is_null() {
        *free_fn = do_free;
    }
}
