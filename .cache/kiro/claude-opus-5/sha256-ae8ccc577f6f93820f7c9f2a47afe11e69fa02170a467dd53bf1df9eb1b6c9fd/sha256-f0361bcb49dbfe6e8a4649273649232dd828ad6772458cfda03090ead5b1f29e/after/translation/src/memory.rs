//! Translation of `src/memory.c`.

use crate::cffi;
use crate::jtypes::{json_free_t, json_malloc_t, json_realloc_t};
use core::ffi::{c_char, c_void};

/* memory function pointers */
static mut do_malloc: json_malloc_t = Some(cffi::malloc);
static mut do_realloc: json_realloc_t = Some(cffi::realloc);
static mut do_free: json_free_t = Some(cffi::free);

#[inline]
unsafe fn get_malloc() -> json_malloc_t {
    unsafe { core::ptr::read(&raw const do_malloc) }
}

#[inline]
unsafe fn get_realloc() -> json_realloc_t {
    unsafe { core::ptr::read(&raw const do_realloc) }
}

#[inline]
unsafe fn get_free() -> json_free_t {
    unsafe { core::ptr::read(&raw const do_free) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_malloc(size: usize) -> *mut c_void {
    unsafe {
        if size == 0 {
            return core::ptr::null_mut();
        }

        (get_malloc().unwrap())(size)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_free(ptr: *mut c_void) {
    unsafe {
        if ptr.is_null() {
            return;
        }

        (get_free().unwrap())(ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_realloc(
    ptr: *mut c_void,
    originalSize: usize,
    newSize: usize,
) -> *mut c_void {
    unsafe {
        if let Some(f) = get_realloc() {
            return f(ptr, newSize);
        }

        // realloc emulation using malloc and free
        if newSize == 0 {
            if !ptr.is_null() {
                (get_free().unwrap())(ptr);
            }

            core::ptr::null_mut()
        } else {
            let new_memory = (get_malloc().unwrap())(newSize);

            if !new_memory.is_null() && !ptr.is_null() {
                let n = if originalSize < newSize {
                    originalSize
                } else {
                    newSize
                };
                core::ptr::copy_nonoverlapping(ptr as *const u8, new_memory as *mut u8, n);

                (get_free().unwrap())(ptr);
            }

            new_memory
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strndup(str_: *const c_char, len: usize) -> *mut c_char {
    unsafe {
        let new_str = jsonp_malloc(len + 1) as *mut c_char;
        if new_str.is_null() {
            return core::ptr::null_mut();
        }

        core::ptr::copy_nonoverlapping(str_ as *const u8, new_str as *mut u8, len);
        *new_str.add(len) = 0;
        new_str
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_set_alloc_funcs(malloc_fn: json_malloc_t, free_fn: json_free_t) {
    unsafe {
        core::ptr::write(&raw mut do_malloc, malloc_fn);
        core::ptr::write(&raw mut do_realloc, None);
        core::ptr::write(&raw mut do_free, free_fn);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_set_alloc_funcs2(
    malloc_fn: json_malloc_t,
    realloc_fn: json_realloc_t,
    free_fn: json_free_t,
) {
    unsafe {
        core::ptr::write(&raw mut do_malloc, malloc_fn);
        core::ptr::write(&raw mut do_realloc, realloc_fn);
        core::ptr::write(&raw mut do_free, free_fn);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs(
    malloc_fn: *mut json_malloc_t,
    free_fn: *mut json_free_t,
) {
    unsafe {
        if !malloc_fn.is_null() {
            *malloc_fn = get_malloc();
        }
        if !free_fn.is_null() {
            *free_fn = get_free();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_get_alloc_funcs2(
    malloc_fn: *mut json_malloc_t,
    realloc_fn: *mut json_realloc_t,
    free_fn: *mut json_free_t,
) {
    unsafe {
        if !malloc_fn.is_null() {
            *malloc_fn = get_malloc();
        }
        if !realloc_fn.is_null() {
            *realloc_fn = get_realloc();
        }
        if !free_fn.is_null() {
            *free_fn = get_free();
        }
    }
}
