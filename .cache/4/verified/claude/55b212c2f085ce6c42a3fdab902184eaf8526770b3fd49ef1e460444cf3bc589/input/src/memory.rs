//! Translation of `src/memory.c`.

#![allow(non_snake_case)]

use crate::types::*;
use core::ffi::{c_char, c_void};

/* memory function pointers */
static mut DO_MALLOC: json_malloc_t = Some(malloc);
static mut DO_REALLOC: json_realloc_t = Some(realloc);
static mut DO_FREE: json_free_t = Some(free);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }

    (DO_MALLOC.unwrap())(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }

    (DO_FREE.unwrap())(ptr);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_realloc(
    ptr: *mut c_void,
    originalSize: usize,
    newSize: usize,
) -> *mut c_void {
    let newMemory: *mut c_void;

    if let Some(f) = DO_REALLOC {
        return f(ptr, newSize);
    }

    // realloc emulation using malloc and free
    if newSize == 0 {
        if !ptr.is_null() {
            (DO_FREE.unwrap())(ptr);
        }

        core::ptr::null_mut()
    } else {
        newMemory = (DO_MALLOC.unwrap())(newSize);

        if !newMemory.is_null() && !ptr.is_null() {
            memcpy(
                newMemory,
                ptr,
                if originalSize < newSize {
                    originalSize
                } else {
                    newSize
                },
            );

            (DO_FREE.unwrap())(ptr);
        }

        newMemory
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strndup(str_: *const c_char, len: usize) -> *mut c_char {
    let new_str: *mut c_char;

    new_str = jsonp_malloc(len + 1) as *mut c_char;
    if new_str.is_null() {
        return core::ptr::null_mut();
    }

    memcpy(new_str as *mut c_void, str_ as *const c_void, len);
    *new_str.add(len) = 0;
    new_str
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_set_alloc_funcs(malloc_fn: json_malloc_t, free_fn: json_free_t) {
    DO_MALLOC = malloc_fn;
    DO_REALLOC = None;
    DO_FREE = free_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_set_alloc_funcs2(
    malloc_fn: json_malloc_t,
    realloc_fn: json_realloc_t,
    free_fn: json_free_t,
) {
    DO_MALLOC = malloc_fn;
    DO_REALLOC = realloc_fn;
    DO_FREE = free_fn;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs(
    malloc_fn: *mut json_malloc_t,
    free_fn: *mut json_free_t,
) {
    if !malloc_fn.is_null() {
        *malloc_fn = DO_MALLOC;
    }
    if !free_fn.is_null() {
        *free_fn = DO_FREE;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs2(
    malloc_fn: *mut json_malloc_t,
    realloc_fn: *mut json_realloc_t,
    free_fn: *mut json_free_t,
) {
    if !malloc_fn.is_null() {
        *malloc_fn = DO_MALLOC;
    }
    if !realloc_fn.is_null() {
        *realloc_fn = DO_REALLOC;
    }
    if !free_fn.is_null() {
        *free_fn = DO_FREE;
    }
}
