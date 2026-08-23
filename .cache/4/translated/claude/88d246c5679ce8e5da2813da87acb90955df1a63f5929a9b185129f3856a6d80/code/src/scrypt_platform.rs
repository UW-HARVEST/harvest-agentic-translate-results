//! Translation of `crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c`.
//!
//! `private/quirks.h` renames every exported function in this file:
//!   `escrypt_alloc_region` -> `_sodium_escrypt_alloc_region`
//!   `escrypt_free_region`  -> `_sodium_escrypt_free_region`
//!   `escrypt_init_local`   -> `_sodium_escrypt_init_local`
//!   `escrypt_free_local`   -> `_sodium_escrypt_free_local`
//!
//! The reference build has no `config.h`, therefore neither `HAVE_MMAP` nor
//! `HAVE_POSIX_MEMALIGN` nor `MAP_ANON` is defined and the `malloc()`-based
//! fallback of `alloc_region()` / `free_region()` is the code that survives the
//! preprocessor.

use core::ffi::{c_int, c_void};
use core::ptr;

/* crypto_scrypt.h:
 *
 *   typedef struct {
 *       void * base, *aligned;
 *       size_t size;
 *   } escrypt_region_t;
 *
 *   typedef escrypt_region_t escrypt_local_t;
 */
#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

/* <errno.h> */
const ENOMEM: c_int = 12;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn __errno_location() -> *mut c_int;
}

/* void *escrypt_alloc_region(escrypt_region_t *region, size_t size) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_alloc_region(
    region: *mut escrypt_region_t,
    size: usize,
) -> *mut c_void {
    let mut base: *mut u8;
    let mut aligned: *mut u8;

    base = ptr::null_mut();
    aligned = base;
    if size.wrapping_add(63) < size {
        *__errno_location() = ENOMEM;
    } else {
        base = malloc(size.wrapping_add(63)) as *mut u8;
        if !base.is_null() {
            aligned = base.wrapping_add(63);
            aligned = ((aligned as usize).wrapping_sub((aligned as usize) & 63)) as *mut u8;
        }
    }
    (*region).base = base as *mut c_void;
    (*region).aligned = aligned as *mut c_void;
    (*region).size = if !base.is_null() { size } else { 0 };

    aligned as *mut c_void
}

/* static inline void init_region(escrypt_region_t *region) */
#[inline(always)]
unsafe fn init_region(region: *mut escrypt_region_t) {
    (*region).base = ptr::null_mut();
    (*region).aligned = (*region).base;
    (*region).size = 0;
}

/* int escrypt_free_region(escrypt_region_t *region) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int {
    if !(*region).base.is_null() {
        free((*region).base);
    }
    init_region(region);

    0
}

/* int escrypt_init_local(escrypt_local_t *local) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int {
    init_region(local);

    0
}

/* int escrypt_free_local(escrypt_local_t *local) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int {
    _sodium_escrypt_free_region(local)
}
