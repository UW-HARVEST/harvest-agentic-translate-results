//! Translation of c_src/libsodium/crypto_aead/aes256gcm/aead_aes256gcm.c
//!
//! The accessors are always compiled. Because
//! `!((HAVE_ARMCRYPTO && NATIVE_LITTLE_ENDIAN) ||
//!    (HAVE_TMMINTRIN_H && HAVE_WMMINTRIN_H))` is TRUE (none of those macros
//! are defined), the ENOSYS stub block is the one compiled.

use core::ffi::{c_int, c_void};

use crate::plat::{set_errno, ENOSYS};

// typedef struct CRYPTO_ALIGN(16) crypto_aead_aes256gcm_state_ {
//     unsigned char opaque[512];
// } crypto_aead_aes256gcm_state;
#[repr(C, align(16))]
pub struct crypto_aead_aes256gcm_state {
    pub opaque: [u8; 512],
}

// Constants from crypto_aead_aes256gcm.h
const CRYPTO_AEAD_AES256GCM_KEYBYTES: usize = 32;
const CRYPTO_AEAD_AES256GCM_NSECBYTES: usize = 0;
const CRYPTO_AEAD_AES256GCM_NPUBBYTES: usize = 12;
const CRYPTO_AEAD_AES256GCM_ABYTES: usize = 16;
// SODIUM_MIN(SODIUM_SIZE_MAX - ABYTES, 16ULL*((1ULL<<32)-2ULL))
const CRYPTO_AEAD_AES256GCM_MESSAGEBYTES_MAX: u64 = {
    let a = (crate::common::SODIUM_SIZE_MAX as u64).wrapping_sub(CRYPTO_AEAD_AES256GCM_ABYTES as u64);
    let b = 16u64 * ((1u64 << 32) - 2u64);
    if a < b {
        a
    } else {
        b
    }
};

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keybytes() -> usize {
    CRYPTO_AEAD_AES256GCM_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_nsecbytes() -> usize {
    CRYPTO_AEAD_AES256GCM_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_npubbytes() -> usize {
    CRYPTO_AEAD_AES256GCM_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_abytes() -> usize {
    CRYPTO_AEAD_AES256GCM_ABYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_statebytes() -> usize {
    (core::mem::size_of::<crypto_aead_aes256gcm_state>() + 15usize) & !15usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_messagebytes_max() -> usize {
    CRYPTO_AEAD_AES256GCM_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_AEAD_AES256GCM_KEYBYTES);
}

// ---- ENOSYS stub block ----
// ENOSYS is defined on Linux, so the `#ifndef ENOSYS #define ENOSYS ENXIO`
// fallback does not apply.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached(
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
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt(
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
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached(
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
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt(
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
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_beforenm(
    st_: *mut crypto_aead_aes256gcm_state,
    k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_afternm(
    c: *mut u8,
    clen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    ad: *const u8,
    adlen: u64,
    nsec: *const u8,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached_afternm(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    mac: *const u8,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_afternm(
    m: *mut u8,
    mlen_p: *mut u64,
    nsec: *mut u8,
    c: *const u8,
    clen: u64,
    ad: *const u8,
    adlen: u64,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_is_available() -> c_int {
    0
}
