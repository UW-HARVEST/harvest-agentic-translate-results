//! Translation of strbuffer.c
use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use crate::types::*;
use core::ffi::{c_char, c_int, c_void};
use core::ptr;

extern "C" {
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

const STRBUFFER_MIN_SIZE: usize = 16;
const STRBUFFER_FACTOR: usize = 2;
const STRBUFFER_SIZE_MAX: usize = usize::MAX;

#[inline]
fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
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
    (*strbuff).value = ptr::null_mut();
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
    (*strbuff).value = ptr::null_mut();
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_append_byte(strbuff: *mut strbuffer_t, byte: c_char) -> c_int {
    strbuffer_append_bytes(strbuff, &byte, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strbuffer_append_bytes(
    strbuff: *mut strbuffer_t,
    data: *const c_char,
    size: usize,
) -> c_int {
    // `wrapping_sub`, not `-`: the C is `size >= strbuff->size - strbuff->length`,
    // plain `size_t` subtraction with no guard.  The library's own use always keeps
    // length < size, but strbuffer_append_bytes() is an exported symbol taking a
    // caller-owned strbuffer_t, so a caller with length > size makes the C wrap to a
    // huge value (the branch is then simply skipped).  Rust's `-` panicked instead.
    //
    // The three SIZE_MAX guards and the two sums below are left as plain `/`, `-`
    // and `+` on purpose: they are the C's own explicit overflow guards
    // (strbuffer.c:66-69) and, thanks to C's left-to-right short-circuiting, each
    // one is only evaluated once the previous has ruled out the overflow, so
    // nothing here can wrap.  `size > SIZE_MAX - 1` rejects size == SIZE_MAX before
    // `SIZE_MAX - 1 - size` runs, and `length > SIZE_MAX - 1 - size` bounds
    // `length + size + 1` by SIZE_MAX exactly.
    if size >= (*strbuff).size.wrapping_sub((*strbuff).length) {
        /* avoid integer overflow */
        if (*strbuff).size > STRBUFFER_SIZE_MAX / STRBUFFER_FACTOR
            || size > STRBUFFER_SIZE_MAX - 1
            || (*strbuff).length > STRBUFFER_SIZE_MAX - 1 - size
        {
            return -1;
        }

        let new_size = max(
            (*strbuff).size * STRBUFFER_FACTOR,
            (*strbuff).length + size + 1,
        );

        let new_value =
            jsonp_realloc((*strbuff).value as *mut c_void, (*strbuff).size, new_size) as *mut c_char;
        if new_value.is_null() {
            return -1;
        }

        (*strbuff).value = new_value;
        (*strbuff).size = new_size;
    }

    memcpy(
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
