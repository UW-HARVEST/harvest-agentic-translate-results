//! Translation of `crypto_aead/aes256gcm/aead_aes256gcm.c`.
//!
//! The reference build defines neither `HAVE_ARMCRYPTO`/`NATIVE_LITTLE_ENDIAN`
//! nor `HAVE_TMMINTRIN_H`/`HAVE_WMMINTRIN_H`, so the
//! `#if !(...)` block at the bottom of the file *is* compiled: every AEAD
//! entry point is a stub that sets `errno = ENOSYS` and returns `-1`, and
//! `crypto_aead_aes256gcm_is_available()` returns `0`.

use core::ffi::{c_int, c_ulonglong, c_void};

extern "C" {
    /* glibc's `errno` accessor. */
    fn __errno_location() -> *mut c_int;

    /* `randombytes.h` */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* <errno.h>: ENOSYS is defined on Linux, so the `#ifndef ENOSYS` fallback to
 * ENXIO is not taken. */
const ENOSYS: c_int = 38;

/* ------------------------------------------------------------------------- */
/* `crypto_aead_aes256gcm.h`                                                 */
/* ------------------------------------------------------------------------- */

/* #define crypto_aead_aes256gcm_KEYBYTES  32U */
const crypto_aead_aes256gcm_KEYBYTES: usize = 32;
/* #define crypto_aead_aes256gcm_NSECBYTES 0U */
const crypto_aead_aes256gcm_NSECBYTES: usize = 0;
/* #define crypto_aead_aes256gcm_NPUBBYTES 12U */
const crypto_aead_aes256gcm_NPUBBYTES: usize = 12;
/* #define crypto_aead_aes256gcm_ABYTES    16U */
const crypto_aead_aes256gcm_ABYTES: usize = 16;
/*
 * #define crypto_aead_aes256gcm_MESSAGEBYTES_MAX \
 *     SODIUM_MIN(SODIUM_SIZE_MAX - crypto_aead_aes256gcm_ABYTES, \
 *                (16ULL * ((1ULL << 32) - 2ULL)))
 *
 * SODIUM_SIZE_MAX == SODIUM_MIN(UINT64_MAX, SIZE_MAX) == 2^64 - 1 here, so the
 * minimum is 16 * (2^32 - 2) == 68719476704.
 */
const crypto_aead_aes256gcm_MESSAGEBYTES_MAX: c_ulonglong = {
    let a: c_ulonglong = (u64::MAX as c_ulonglong) - (crypto_aead_aes256gcm_ABYTES as c_ulonglong);
    let b: c_ulonglong = 16u64 * ((1u64 << 32) - 2u64);
    if a < b {
        a
    } else {
        b
    }
};

/*
 * typedef struct CRYPTO_ALIGN(16) crypto_aead_aes256gcm_state_ {
 *     unsigned char opaque[512];
 * } crypto_aead_aes256gcm_state;
 */
#[repr(C, align(16))]
pub struct crypto_aead_aes256gcm_state {
    pub opaque: [u8; 512],
}

/* ------------------------------------------------------------------------- */

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

/* ------------------------------------------------------------------------- */
/* `#if !((HAVE_ARMCRYPTO && NATIVE_LITTLE_ENDIAN) ||                        */
/*        (HAVE_TMMINTRIN_H && HAVE_WMMINTRIN_H))` — taken in this build.    */
/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let _ = (c, mac, maclen_p, m, mlen, ad, adlen, nsec, npub, k);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt(
    c: *mut u8,
    clen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let _ = (c, clen_p, m, mlen, ad, adlen, nsec, npub, k);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let _ = (m, nsec, c, clen, mac, ad, adlen, npub, k);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt(
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    k: *const u8,
) -> c_int {
    let _ = (m, mlen_p, nsec, c, clen, ad, adlen, npub, k);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_beforenm(
    st_: *mut crypto_aead_aes256gcm_state,
    k: *const u8,
) -> c_int {
    let _ = (st_, k);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    maclen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    let _ = (c, mac, maclen_p, m, mlen, ad, adlen, nsec, npub, st_);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_encrypt_afternm(
    c: *mut u8,
    clen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    nsec: *const u8,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    let _ = (c, clen_p, m, mlen, ad, adlen, nsec, npub, st_);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_detached_afternm(
    m: *mut u8,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    mac: *const u8,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    let _ = (m, nsec, c, clen, mac, ad, adlen, npub, st_);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_decrypt_afternm(
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    nsec: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    ad: *const u8,
    adlen: c_ulonglong,
    npub: *const u8,
    st_: *const crypto_aead_aes256gcm_state,
) -> c_int {
    let _ = (m, mlen_p, nsec, c, clen, ad, adlen, npub, st_);
    *__errno_location() = ENOSYS;
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_aead_aes256gcm_is_available() -> c_int {
    0
}
