//! Translation of c_src/libsodium/crypto_aead/aegis256/aead_aegis256.c
//!
//! HAVE_ARMCRYPTO / HAVE_AVXINTRIN_H / HAVE_WMMINTRIN_H are all undefined, so
//! only the soft implementation exists.

use core::ffi::{c_int, c_void};

use crate::common::SODIUM_SIZE_MAX;
use crate::sodium::core::sodium_misuse;

// Constants from crypto_aead_aegis256.h
const CRYPTO_AEAD_AEGIS256_KEYBYTES: usize = 32;
const CRYPTO_AEAD_AEGIS256_NSECBYTES: usize = 0;
const CRYPTO_AEAD_AEGIS256_NPUBBYTES: usize = 32;
const CRYPTO_AEAD_AEGIS256_ABYTES: usize = 32;
// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, (1ULL << 61) - 1)
const CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX: u64 = {
    let a = (SODIUM_SIZE_MAX as u64).wrapping_sub(CRYPTO_AEAD_AEGIS256_ABYTES as u64);
    let b = (1u64 << 61) - 1;
    if a < b {
        a
    } else {
        b
    }
};

type EncryptDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;

type DecryptDetachedFn = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    usize,
    *const u8,
    *const u8,
) -> c_int;

#[repr(C)]
struct aegis256_implementation {
    encrypt_detached: Option<EncryptDetachedFn>,
    decrypt_detached: Option<DecryptDetachedFn>,
}
unsafe impl Sync for aegis256_implementation {}

extern "C" {
    static aegis256_soft_implementation: aegis256_implementation;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// static const aegis256_implementation *implementation = &aegis256_soft_implementation;
static mut implementation: *const aegis256_implementation = core::ptr::null();

#[inline]
unsafe fn impl_ptr() -> *const aegis256_implementation {
    if implementation.is_null() {
        implementation = &aegis256_soft_implementation as *const aegis256_implementation;
    }
    implementation
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_keybytes() -> usize {
    CRYPTO_AEAD_AEGIS256_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_nsecbytes() -> usize {
    CRYPTO_AEAD_AEGIS256_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_npubbytes() -> usize {
    CRYPTO_AEAD_AEGIS256_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_abytes() -> usize {
    CRYPTO_AEAD_AEGIS256_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_messagebytes_max() -> usize {
    CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_AEAD_AEGIS256_KEYBYTES);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt(
    c: *mut u8,
    clen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut clen: u64 = 0u64;
    let ret: c_int;

    if mlen > CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ret = crypto_aead_aegis256_encrypt_detached(
        c,
        c.add(mlen as usize),
        core::ptr::null_mut(),
        m,
        mlen,
        ad,
        adlen,
        nsec,
        npub,
        k,
    );
    if !clen_p.is_null() {
        if ret == 0 {
            clen = mlen + CRYPTO_AEAD_AEGIS256_ABYTES as u64;
        }
        *clen_p = clen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt(
    m: *mut u8,
    mlen_p: *mut u64,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let mut mlen: u64 = 0u64;
    let mut ret: c_int = -1;

    if clen >= CRYPTO_AEAD_AEGIS256_ABYTES as u64 {
        ret = crypto_aead_aegis256_decrypt_detached(
            m,
            nsec,
            c,
            clen - CRYPTO_AEAD_AEGIS256_ABYTES as u64,
            c.add((clen - CRYPTO_AEAD_AEGIS256_ABYTES as u64) as usize),
            ad,
            adlen,
            npub,
            k,
        );
    }
    if !mlen_p.is_null() {
        if ret == 0 {
            mlen = clen - CRYPTO_AEAD_AEGIS256_ABYTES as u64;
        }
        *mlen_p = mlen;
    }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let maclen: usize = CRYPTO_AEAD_AEGIS256_ABYTES;

    let _ = nsec;
    if !maclen_p.is_null() {
        *maclen_p = maclen as u64;
    }
    if mlen > CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX || adlen > CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX
    {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    ((*impl_ptr()).encrypt_detached.unwrap_unchecked())(
        c,
        mac,
        maclen,
        m,
        mlen as usize,
        ad,
        adlen as usize,
        npub,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aegis256_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let maclen: usize = CRYPTO_AEAD_AEGIS256_ABYTES;

    let _ = nsec;
    if clen > CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX || adlen > CRYPTO_AEAD_AEGIS256_MESSAGEBYTES_MAX
    {
        return -1; /* LCOV_EXCL_LINE */
    }
    ((*impl_ptr()).decrypt_detached.unwrap_unchecked())(
        m,
        c,
        clen as usize,
        mac,
        maclen,
        ad,
        adlen as usize,
        npub,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_aead_aegis256_pick_best_implementation() -> c_int {
    implementation = &aegis256_soft_implementation as *const aegis256_implementation;

    // HAVE_ARMCRYPTO / HAVE_AVXINTRIN_H / HAVE_WMMINTRIN_H undefined.
    0 /* LCOV_EXCL_LINE */
}
