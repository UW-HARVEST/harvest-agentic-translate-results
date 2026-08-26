//! Translation of c_src/src/strbuffer.c
use crate::libc;
use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use std::ffi::{c_char, c_int, c_void};

pub const STRBUFFER_MIN_SIZE: usize = 16;
pub const STRBUFFER_FACTOR: usize = 2;
pub const STRBUFFER_SIZE_MAX: usize = usize::MAX;

#[repr(C)]
pub struct strbuffer_t {
    pub value: *mut c_char,
    pub length: usize,
    pub size: usize,
}

impl strbuffer_t {
    pub const fn new() -> strbuffer_t {
        strbuffer_t {
            value: std::ptr::null_mut(),
            length: 0,
            size: 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_init(strbuff: *mut strbuffer_t) -> c_int {
    (*strbuff).size = STRBUFFER_MIN_SIZE;
    (*strbuff).length = 0;

    (*strbuff).value = jsonp_malloc((*strbuff).size) as *mut c_char;
    if (*strbuff).value.is_null() {
        return -1;
    }

    /* initialize to empty */
    *(*strbuff).value = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_close(strbuff: *mut strbuffer_t) {
    if !(*strbuff).value.is_null() {
        jsonp_free((*strbuff).value as *mut c_void);
    }

    (*strbuff).size = 0;
    (*strbuff).length = 0;
    (*strbuff).value = std::ptr::null_mut();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_clear(strbuff: *mut strbuffer_t) {
    (*strbuff).length = 0;
    *(*strbuff).value = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_value(strbuff: *const strbuffer_t) -> *const c_char {
    (*strbuff).value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_steal_value(strbuff: *mut strbuffer_t) -> *mut c_char {
    let result = (*strbuff).value;
    (*strbuff).value = std::ptr::null_mut();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_append_byte(strbuff: *mut strbuffer_t, byte: c_char) -> c_int {
    let b = byte;
    strbuffer_append_bytes(strbuff, &b as *const c_char, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_append_bytes(
    strbuff: *mut strbuffer_t,
    data: *const c_char,
    size: usize,
) -> c_int {
    if size >= (*strbuff).size.wrapping_sub((*strbuff).length) {
        let new_size: usize;
        let new_value: *mut c_char;

        /* avoid integer overflow */
        if (*strbuff).size > STRBUFFER_SIZE_MAX / STRBUFFER_FACTOR
            || size > STRBUFFER_SIZE_MAX - 1
            || (*strbuff).length > STRBUFFER_SIZE_MAX - 1 - size
        {
            return -1;
        }

        new_size = std::cmp::max(
            (*strbuff).size * STRBUFFER_FACTOR,
            (*strbuff).length + size + 1,
        );

        new_value =
            jsonp_realloc((*strbuff).value as *mut c_void, (*strbuff).size, new_size) as *mut c_char;
        if new_value.is_null() {
            return -1;
        }

        (*strbuff).value = new_value;
        (*strbuff).size = new_size;
    }

    libc::memcpy(
        (*strbuff).value.add((*strbuff).length) as *mut c_void,
        data as *const c_void,
        size,
    );
    (*strbuff).length += size;
    *(*strbuff).value.add((*strbuff).length) = 0;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_pop(strbuff: *mut strbuffer_t) -> c_char {
    if (*strbuff).length > 0 {
        (*strbuff).length -= 1;
        let c = *(*strbuff).value.add((*strbuff).length);
        *(*strbuff).value.add((*strbuff).length) = 0;
        c
    } else {
        0
    }
}
