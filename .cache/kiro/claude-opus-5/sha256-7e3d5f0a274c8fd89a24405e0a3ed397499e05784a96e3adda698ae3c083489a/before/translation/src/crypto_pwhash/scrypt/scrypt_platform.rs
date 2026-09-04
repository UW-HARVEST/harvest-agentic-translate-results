//! Translation of crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c
//! and the shared types from crypto_pwhash/scryptsalsa208sha256/crypto_scrypt.h
//!
//! Build facts: HAVE_MMAP / HAVE_POSIX_MEMALIGN / HAVE_SYS_MMAN_H are all
//! undefined, so the plain malloc()/free() fallback path is taken.

use core::ffi::{c_int, c_void};

use crate::ENOMEM;

/* ---- shared types from crypto_scrypt.h ---- */

#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

pub type escrypt_kdf_t = unsafe extern "C" fn(
    __local: *mut escrypt_local_t,
    __passwd: *const u8,
    __passwdlen: usize,
    __salt: *const u8,
    __saltlen: usize,
    __N: u64,
    __r: u32,
    __p: u32,
    __buf: *mut u8,
    __buflen: usize,
) -> c_int;

/* ---- scrypt_platform.c ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_alloc_region(
    region: *mut escrypt_region_t,
    size: usize,
) -> *mut c_void {
    let mut base: *mut u8;
    let mut aligned: *mut u8;

    base = core::ptr::null_mut();
    aligned = core::ptr::null_mut();
    if size + 63 < size {
        crate::set_errno(ENOMEM);
    } else {
        base = libc::malloc(size + 63) as *mut u8;
        if !base.is_null() {
            aligned = base.add(63);
            aligned = aligned.wrapping_sub((aligned as usize & 63) as usize);
        }
    }
    (*region).base = base as *mut c_void;
    (*region).aligned = aligned as *mut c_void;
    (*region).size = if !base.is_null() { size } else { 0 };

    aligned as *mut c_void
}

#[inline]
unsafe fn init_region(region: *mut escrypt_region_t) {
    (*region).base = core::ptr::null_mut();
    (*region).aligned = core::ptr::null_mut();
    (*region).size = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_region(region: *mut escrypt_region_t) -> c_int {
    if !(*region).base.is_null() {
        libc::free((*region).base);
    }
    init_region(region);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_init_local(local: *mut escrypt_local_t) -> c_int {
    init_region(local);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_free_local(local: *mut escrypt_local_t) -> c_int {
    _sodium_escrypt_free_region(local)
}
