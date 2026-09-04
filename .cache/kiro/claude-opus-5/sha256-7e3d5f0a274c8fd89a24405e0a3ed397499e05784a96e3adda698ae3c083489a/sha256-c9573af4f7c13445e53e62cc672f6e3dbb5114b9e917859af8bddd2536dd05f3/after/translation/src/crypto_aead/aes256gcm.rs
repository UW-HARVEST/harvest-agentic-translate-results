//! Translation of `crypto_aead/aes256gcm/aead_aes256gcm.c`.
//!
//! Build config facts: no `HAVE_*` macros are defined, so the aesni/armcrypto
//! variants are entirely compiled out. Only the portable stub survives:
//! `crypto_aead_aes256gcm_is_available()` returns 0 and every AEAD operation
//! sets `errno = ENOSYS` and returns -1. The size/keygen accessors remain.

use crate::randombytes::randombytes_buf;

// ---- constants from include/sodium/crypto_aead_aes256gcm.h ----
pub const crypto_aead_aes256gcm_KEYBYTES: usize = 32;
pub const crypto_aead_aes256gcm_NSECBYTES: usize = 0;
pub const crypto_aead_aes256gcm_NPUBBYTES: usize = 12;
pub const crypto_aead_aes256gcm_ABYTES: usize = 16;
// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, (16ULL * ((1ULL << 32) - 2ULL)))
pub const crypto_aead_aes256gcm_MESSAGEBYTES_MAX: usize = {
    let a = usize::MAX - crypto_aead_aes256gcm_ABYTES;
    let b = (16u64 * ((1u64 << 32) - 2u64)) as usize;
    if a < b {
        a
    } else {
        b
    }
};

// typedef struct CRYPTO_ALIGN(16) crypto_aead_aes256gcm_state_ {
//     unsigned char opaque[512];
// } crypto_aead_aes256gcm_state;
#[repr(C, align(16))]
pub struct crypto_aead_aes256gcm_state {
    pub opaque: [u8; 512],
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_keybytes() -> usize {
    crypto_aead_aes256gcm_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_nsecbytes() -> usize {
    crypto_aead_aes256gcm_NSECBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_npubbytes() -> usize {
    crypto_aead_aes256gcm_NPUBBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_abytes() -> usize {
    crypto_aead_aes256gcm_ABYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_statebytes() -> usize {
    (core::mem::size_of::<crypto_aead_aes256gcm_state>() + 15usize) & !15usize
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_messagebytes_max() -> usize {
    crypto_aead_aes256gcm_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_aead_aes256gcm_KEYBYTES);
}

// ---------------------------------------------------------------------------
// Portable stub path (no HAVE_ARMCRYPTO / HAVE_TMMINTRIN_H|HAVE_WMMINTRIN_H).
// `ENOSYS` is defined on the target, so the C `#ifndef ENOSYS` fallback to
// `ENXIO` is not taken.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt(
    c: *mut u8,
    clen_p: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: core::ffi::c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt(
    m: *mut u8,
    mlen_p: *mut core::ffi::c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_beforenm(
    st_: *mut crypto_aead_aes256gcm_state,
    k: *const u8,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_afternm(
    c: *mut u8,
    clen_p: *mut core::ffi::c_ulonglong,
    m: *const u8,
    mlen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached_afternm(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: core::ffi::c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_afternm(
    m: *mut u8,
    mlen_p: *mut core::ffi::c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: core::ffi::c_ulonglong,
    ad: *const u8,
    adlen: core::ffi::c_ulonglong,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> core::ffi::c_int {
    crate::set_errno(crate::ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_is_available() -> core::ffi::c_int {
    0
}
