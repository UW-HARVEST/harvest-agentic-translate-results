//! Translated from crypto_aead/aes256gcm/aead_aes256gcm.c (reference path: no HW AES).
//! The reference build has no HAVE_TMMINTRIN_H/WMMINTRIN_H/ARMCRYPTO, so all
//! operational functions return -1 with errno = ENOSYS.
use core::ffi::{c_int, c_void};

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn __errno_location() -> *mut c_int;
}

// From <errno.h> on Linux.
const ENOSYS: c_int = 38;

const KEYBYTES: usize = 32;
const NSECBYTES: usize = 0;
const NPUBBYTES: usize = 12;
const ABYTES: usize = 16;
// min(SODIUM_SIZE_MAX - ABYTES, 16*(2^32-2))
const MESSAGEBYTES_MAX: u64 = 16 * ((1u64 << 32) - 2);
// state is CRYPTO_ALIGN(16) { unsigned char opaque[512]; }
const STATE_SIZE: usize = 512;

#[inline]
unsafe fn set_enosys() {
    *__errno_location() = ENOSYS;
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_keybytes() -> usize {
    KEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_nsecbytes() -> usize {
    NSECBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_npubbytes() -> usize {
    NPUBBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_abytes() -> usize {
    ABYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_statebytes() -> usize {
    (STATE_SIZE + 15) & !15
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_messagebytes_max() -> usize {
    MESSAGEBYTES_MAX as usize
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, KEYBYTES);
}

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
    set_enosys();
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
    set_enosys();
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
    set_enosys();
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
    set_enosys();
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_beforenm(
    _st_: *mut c_void,
    _k: *const u8,
) -> c_int {
    set_enosys();
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
    _st_: *const c_void,
) -> c_int {
    set_enosys();
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
    _st_: *const c_void,
) -> c_int {
    set_enosys();
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
    _st_: *const c_void,
) -> c_int {
    set_enosys();
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
    _st_: *const c_void,
) -> c_int {
    set_enosys();
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_aead_aes256gcm_is_available() -> c_int {
    0
}
