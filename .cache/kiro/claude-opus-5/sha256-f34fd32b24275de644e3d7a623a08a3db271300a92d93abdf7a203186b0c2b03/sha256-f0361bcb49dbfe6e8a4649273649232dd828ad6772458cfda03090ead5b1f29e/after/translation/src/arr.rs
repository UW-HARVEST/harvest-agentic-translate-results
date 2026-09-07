//! Dynamic array back-end: `stbds_arrgrowf` / `stbds_arrfreef`.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;

use crate::c;
use crate::types::*;

/// `void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut c_void {
    let mut min_cap = min_cap;

    // `size_t min_len = stbds_arrlen(a) + addlen;`
    let min_len = (stbds_arrlen(a) as usize).wrapping_add(addlen);

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
    let raw = c::realloc(
        old,
        elemsize
            .wrapping_mul(min_cap)
            .wrapping_add(size_of::<ArrayHeader>()),
    );
    let b = (raw as *mut u8).wrapping_add(size_of::<ArrayHeader>()) as *mut c_void;

    if a.is_null() {
        (*stbds_header(b)).length = 0;
        (*stbds_header(b)).hash_table = ptr::null_mut();
        (*stbds_header(b)).temp = 0;
    }
    (*stbds_header(b)).capacity = min_cap;

    b
}

/// `void stbds_arrfreef(void *a)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_arrfreef(a: *mut c_void) {
    c::free(stbds_header(a) as *mut c_void);
}
