//! Translation of c_src/libsodium/crypto_pwhash/scryptsalsa208sha256/scrypt_platform.c

use core::ffi::{c_int, c_void};

// escrypt_region_t / escrypt_local_t from crypto_scrypt.h.
#[repr(C)]
pub struct escrypt_region_t {
    pub base: *mut c_void,
    pub aligned: *mut c_void,
    pub size: usize,
}

pub type escrypt_local_t = escrypt_region_t;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_escrypt_alloc_region(
    region: *mut escrypt_region_t,
    size: usize,
) -> *mut c_void {
    let mut base: *mut u8;
    let mut aligned: *mut u8;
    // HAVE_MMAP / HAVE_POSIX_MEMALIGN undefined: plain malloc fallback branch.
    base = core::ptr::null_mut();
    aligned = core::ptr::null_mut();
    if size.wrapping_add(63) < size {
        crate::plat::set_errno(crate::plat::ENOMEM);
    } else {
        base = malloc(size + 63) as *mut u8;
        if !base.is_null() {
            aligned = base.add(63);
            aligned = aligned.wrapping_sub((aligned as usize) & 63);
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
        // HAVE_MMAP undefined: plain free fallback branch.
        free((*region).base);
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
