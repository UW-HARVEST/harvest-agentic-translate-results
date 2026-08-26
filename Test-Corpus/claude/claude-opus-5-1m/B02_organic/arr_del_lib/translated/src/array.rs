//! Dynamic array support: `stbds_arrgrowf` / `stbds_arrfreef`.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::null_mut;

use crate::*;

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
    // stbds_array_header temp={0}; (void) sizeof(temp);
    let mut min_cap = min_cap;
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= stbds_arrcap(a) {
        return a;
    }

    if min_cap < stbds_arrcap(a).wrapping_mul(2) {
        min_cap = stbds_arrcap(a).wrapping_mul(2);
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old: *mut c_void = if !a.is_null() {
        stbds_header(a) as *mut c_void
    } else {
        null_mut()
    };

    let mut b = STBDS_REALLOC(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<stbds_array_header>()),
    );
    b = (b as *mut u8).wrapping_add(size_of::<stbds_array_header>()) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = null_mut();
        (*stbds_header(b)).temp = 0;
    } else {
        // STBDS_STATS(++stbds_array_grow);
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

/// ```c
/// void stbds_arrfreef(void *a)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    STBDS_FREE(stbds_header(a) as *mut c_void);
}
