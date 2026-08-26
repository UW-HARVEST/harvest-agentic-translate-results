//! Translation of `crypto_aead/aes256gcm/aead_aes256gcm.c`.
//!
//! The reference build defines neither `HAVE_ARMCRYPTO`/`NATIVE_LITTLE_ENDIAN`
//! nor `HAVE_TMMINTRIN_H`/`HAVE_WMMINTRIN_H`, so `aead_aes256gcm_aesni.c` and
//! `aead_aes256gcm_armcrypto.c` produce no symbols at all and this file
//! compiles the
//!
//! ```c
//! #if !((defined(HAVE_ARMCRYPTO) && defined(NATIVE_LITTLE_ENDIAN)) || \
//!       (defined(HAVE_TMMINTRIN_H) && defined(HAVE_WMMINTRIN_H)))
//! ```
//!
//! "unavailable" branch: every AES256-GCM operation sets `errno = ENOSYS` and
//! returns `-1`, and `crypto_aead_aes256gcm_is_available()` returns `0`.  None
//! of them call `sodium_misuse()` in this configuration.
//!
//! The size/keygen accessors above that `#if` are compiled unconditionally.

use core::ffi::{c_int, c_void};

use crate::common::{ENOSYS, set_errno};
use crate::randombytes::randombytes_buf;

// ---------------------------------------------------------------------------
// constants (include/sodium/crypto_aead_aes256gcm.h)
// ---------------------------------------------------------------------------

pub const crypto_aead_aes256gcm_KEYBYTES: usize = 32;
pub const crypto_aead_aes256gcm_NSECBYTES: usize = 0;
pub const crypto_aead_aes256gcm_NPUBBYTES: usize = 12;
pub const crypto_aead_aes256gcm_ABYTES: usize = 16;

/// `SODIUM_MIN(SODIUM_SIZE_MAX - crypto_aead_aes256gcm_ABYTES,
///             (16ULL * ((1ULL << 32) - 2ULL)))`
pub const crypto_aead_aes256gcm_MESSAGEBYTES_MAX: u64 = {
    let a = crate::common::SODIUM_SIZE_MAX - crypto_aead_aes256gcm_ABYTES as u64;
    let b = 16u64 * ((1u64 << 32) - 2u64);
    if a < b { a } else { b }
};

/// ```c
/// typedef struct CRYPTO_ALIGN(16) crypto_aead_aes256gcm_state_ {
///     unsigned char opaque[512];
/// } crypto_aead_aes256gcm_state;
/// ```
#[repr(C, align(16))]
pub struct crypto_aead_aes256gcm_state {
    pub opaque: [u8; 512],
}

// ---------------------------------------------------------------------------
// unconditionally compiled accessors
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keybytes() -> usize {
    crypto_aead_aes256gcm_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_nsecbytes() -> usize {
    crypto_aead_aes256gcm_NSECBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_npubbytes() -> usize {
    crypto_aead_aes256gcm_NPUBBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_abytes() -> usize {
    crypto_aead_aes256gcm_ABYTES
}

/// `return (sizeof(crypto_aead_aes256gcm_state) + (size_t) 15U) & ~(size_t) 15U;`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_statebytes() -> usize {
    (core::mem::size_of::<crypto_aead_aes256gcm_state>() + 15usize) & !15usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_messagebytes_max() -> usize {
    crypto_aead_aes256gcm_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_aead_aes256gcm_KEYBYTES);
}

// ---------------------------------------------------------------------------
// "unavailable" branch: errno = ENOSYS; return -1;
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached(
    _c: *mut u8,
    _mac: *mut u8,
    _maclen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt(
    _c: *mut u8,
    _clen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached(
    _m: *mut u8,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _mac: *const u8,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt(
    _m: *mut u8,
    _mlen_p: *mut u64,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_beforenm(
    _st_: *mut crypto_aead_aes256gcm_state,
    _k: *const u8,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached_afternm(
    _c: *mut u8,
    _mac: *mut u8,
    _maclen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_afternm(
    _c: *mut u8,
    _clen_p: *mut u64,
    _m: *const u8,
    _mlen: u64,
    _ad: *const u8,
    _adlen: u64,
    _nsec: *const u8,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached_afternm(
    _m: *mut u8,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _mac: *const u8,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_afternm(
    _m: *mut u8,
    _mlen_p: *mut u64,
    _nsec: *mut u8,
    _c: *const u8,
    _clen: u64,
    _ad: *const u8,
    _adlen: u64,
    _npub: *const u8,
    _st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    set_errno(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_is_available() -> c_int {
    0
}
