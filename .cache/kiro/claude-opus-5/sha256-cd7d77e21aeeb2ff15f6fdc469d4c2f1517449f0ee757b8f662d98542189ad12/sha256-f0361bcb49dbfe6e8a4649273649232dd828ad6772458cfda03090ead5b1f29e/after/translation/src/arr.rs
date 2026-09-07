//! Dynamic array support: `stbds_arrgrowf` / `stbds_arrfreef`.

use core::ffi::c_void;
use core::ptr;

use crate::{arrcap, arrlen, free, header, realloc, HEADER_SIZE};

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
    let mut min_cap = min_cap;
    let b: *mut c_void;
    // size_t min_len = stbds_arrlen(a) + addlen;
    let min_len = (arrlen(a) as usize).wrapping_add(addlen);

    if min_len > min_cap {
        min_cap = min_len;
    }

    if min_cap <= arrcap(a) {
        return a;
    }

    if min_cap < 2usize.wrapping_mul(arrcap(a)) {
        min_cap = 2usize.wrapping_mul(arrcap(a));
    } else if min_cap < 4 {
        min_cap = 4;
    }

    let old: *mut c_void = if !a.is_null() {
        header(a) as *mut c_void
    } else {
        ptr::null_mut()
    };

    let raw = realloc(
        old,
        elemsize.wrapping_mul(min_cap).wrapping_add(HEADER_SIZE),
    );
    b = (raw as *mut u8).wrapping_add(HEADER_SIZE) as *mut c_void;

    if a.is_null() {
        (*header(b)).length = 0;
        (*header(b)).hash_table = ptr::null_mut();
        (*header(b)).temp = 0;
    }
    (*header(b)).capacity = min_cap;

    b
}

/// ```c
/// void stbds_arrfreef(void *a)
/// ```
///
/// Note: the C version unconditionally frees `stbds_header(a)`, which is a
/// wild pointer when `a == NULL`. That behaviour is preserved.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    free(header(a) as *mut c_void);
}
