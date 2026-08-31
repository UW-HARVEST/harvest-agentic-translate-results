//! Translation of `src/memory.c`.

use core::ffi::{c_char, c_void};

use crate::ffi;
use crate::jansson::{json_free_t, json_malloc_t, json_realloc_t};

/* Default to the libc allocator, exactly like the C sources do. The function
   pointers must compare equal to `malloc`/`realloc`/`free` so that
   json_get_alloc_funcs() reports the same values as the C library. */
static mut DO_MALLOC: json_malloc_t = Some(ffi::malloc);
static mut DO_REALLOC: json_realloc_t = Some(ffi::realloc);
static mut DO_FREE: json_free_t = Some(ffi::free);

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

    (DO_FREE.unwrap())(ptr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_realloc(
    ptr: *mut c_void,
    original_size: usize,
    new_size: usize,
) -> *mut c_void {
    let new_memory: *mut c_void;

    if let Some(f) = DO_REALLOC {
        return f(ptr, new_size);
    }

    // realloc emulation using malloc and free
    if new_size == 0 {
        if !ptr.is_null() {
            (DO_FREE.unwrap())(ptr);
        }

        core::ptr::null_mut()
    } else {
        new_memory = (DO_MALLOC.unwrap())(new_size);

        if !new_memory.is_null() && !ptr.is_null() {
            ffi::memcpy(
                new_memory,
                ptr,
                if original_size < new_size {
                    original_size
                } else {
                    new_size
                },
            );

            (DO_FREE.unwrap())(ptr);
        }

        new_memory
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strndup(str_: *const c_char, len: usize) -> *mut c_char {
    let new_str = jsonp_malloc(len + 1) as *mut c_char;
    if new_str.is_null() {
        return core::ptr::null_mut();
    }

    ffi::memcpy(new_str as *mut c_void, str_ as *const c_void, len);
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
