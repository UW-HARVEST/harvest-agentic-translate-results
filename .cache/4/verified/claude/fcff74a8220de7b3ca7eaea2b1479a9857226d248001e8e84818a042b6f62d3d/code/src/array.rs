//! Dynamic array support (`stbds_arrgrowf`, `stbds_arrfreef`).

use core::ffi::c_void;

use crate::ffi::*;

/// ```c
/// void *stbds_arrgrowf(void *a, size_t elemsize, size_t addlen, size_t min_cap)
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_arrgrowf(
    a: *mut c_void,
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
) -> *mut c_void {
    unsafe {
        let mut min_cap = min_cap;
        let b: *mut c_void;
        // size_t min_len = stbds_arrlen(a) + addlen;
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

        b = realloc(
            if !a.is_null() {
                stbds_header(a) as *mut c_void
            } else {
                core::ptr::null_mut()
            },
            elemsize
                .wrapping_mul(min_cap)
                .wrapping_add(core::mem::size_of::<StbdsArrayHeader>()),
        );
        let b = (b as *mut u8).wrapping_add(core::mem::size_of::<StbdsArrayHeader>()) as *mut c_void;
        if a.is_null() {
            (*stbds_header(b)).length = 0;
            (*stbds_header(b)).hash_table = core::ptr::null_mut();
            (*stbds_header(b)).temp = 0;
        }
        (*stbds_header(b)).capacity = min_cap;

        b
    }
}

/// ```c
/// void stbds_arrfreef(void *a) { STBDS_FREE(NULL, stbds_header(a)); }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn stbds_arrfreef(a: *mut c_void) {
    unsafe {
        free(stbds_header(a) as *mut c_void);
    }
}
