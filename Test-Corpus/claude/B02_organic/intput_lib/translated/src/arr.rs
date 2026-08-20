//! Dynamic array growth helpers, translated from `c_src/src/lib.c`.

use core::ffi::{c_char, c_void};
use core::ptr;

use crate::cffi::{free, realloc};
use crate::types::*;

/// ```c
/// void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut c_void {
    // stbds_array_header temp={0}; (void) sizeof(temp);   -- unused in C
    let mut min_cap = min_cap;
    let b: *mut c_void;
    let min_len: usize = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < 2usize.wrapping_mul(stbds_arrcap(a)) {
        min_cap = 2usize.wrapping_mul(stbds_arrcap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };
    b = realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    );
    let b = (b as *mut c_char).wrapping_add(HEADER_SIZE) as *mut c_void;
    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    } else {
        // STBDS_STATS(++stbds_array_grow);  -- compiled out
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

/// ```c
/// void stbds_arrfreef(void *a) { STBDS_FREE(NULL, stbds_header(a)); }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(stbds_header(a) as *mut c_void);
}
