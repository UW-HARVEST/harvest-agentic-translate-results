//! Translation of `crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c`.
//!
//! The reference build defines no `HAVE_*` macro:
//!  * `HAVE_SYS_MMAN_H` undefined, so `<sys/mman.h>` is never included and
//!    `MAP_ANON` therefore stays undefined; combined with an undefined
//!    `HAVE_MMAP` the `#if defined(MAP_ANON) && defined(HAVE_MMAP)` arm is out.
//!  * `HAVE_POSIX_MEMALIGN` undefined, so the `#elif` arm is out too.
//!
//! Only the plain `malloc(size + 63)` / manual 64-byte alignment fallback is
//! compiled, and `escrypt_free_region()` uses `free()`.

#![allow(unsafe_op_in_unsafe_fn)]

use core::ffi::{c_int, c_void};
use core::ptr;

use crate::common::{ENOMEM, free, malloc, set_errno};
use crate::crypto_pwhash::scrypt::common::{escrypt_local_t, escrypt_region_t};

/// ```c
/// void *
/// escrypt_alloc_region(escrypt_region_t *region, size_t size)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_alloc_region(
    region: *mut escrypt_region_t,
    size: usize,
) -> *mut c_void {
    let mut base: *mut u8;
    let mut aligned: *mut u8;

    // base = aligned = NULL;
    aligned = ptr::null_mut();
    base = aligned;
    if size.wrapping_add(63) < size {
        set_errno(ENOMEM);
    } else {
        base = malloc(size.wrapping_add(63)) as *mut u8;
        if !base.is_null() {
            aligned = base.wrapping_add(63);
            aligned = aligned.wrapping_sub((aligned as usize) & 63);
        }
    }
    (*region).base = base as *mut c_void;
    (*region).aligned = aligned as *mut c_void;
    (*region).size = if !base.is_null() { size } else { 0 };

    aligned as *mut c_void
}

/// ```c
/// static inline void
/// init_region(escrypt_region_t *region)
/// ```
#[inline]
unsafe fn init_region(region: *mut escrypt_region_t) {
    // region->base = region->aligned = NULL;
    (*region).aligned = ptr::null_mut();
    (*region).base = (*region).aligned;
    (*region).size = 0;
}

/// ```c
/// int
/// escrypt_free_region(escrypt_region_t *region)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int {
    if !(*region).base.is_null() {
        free((*region).base);
    }
    init_region(region);

    0
}

/// ```c
/// int
/// escrypt_init_local(escrypt_local_t *local)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int {
    init_region(local);

    0
}

/// ```c
/// int
/// escrypt_free_local(escrypt_local_t *local)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int {
    _sodium_escrypt_free_region(local)
}
