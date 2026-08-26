//! Translation of `crypto_box/crypto_box.c`.

use crate::common::*;
use core::ffi::{c_char, c_int, c_ulonglong};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_box.h                                */
/* ------------------------------------------------------------------------- */

/* #define crypto_box_SEEDBYTES ... (32U) */
const crypto_box_SEEDBYTES: usize = 32;
/* #define crypto_box_PUBLICKEYBYTES ... (32U) */
const crypto_box_PUBLICKEYBYTES: usize = 32;
/* #define crypto_box_SECRETKEYBYTES ... (32U) */
const crypto_box_SECRETKEYBYTES: usize = 32;
/* #define crypto_box_BEFORENMBYTES ... (32U) */
const crypto_box_BEFORENMBYTES: usize = 32;
/* #define crypto_box_NONCEBYTES ... (24U) */
const crypto_box_NONCEBYTES: usize = 24;
/* #define crypto_box_MACBYTES ... (16U) */
const crypto_box_MACBYTES: usize = 16;
/* #define crypto_box_BOXZEROBYTES ... (16U) */
const crypto_box_BOXZEROBYTES: usize = 16;
/* #define crypto_box_ZEROBYTES ... (BOXZEROBYTES + MACBYTES) */
const crypto_box_ZEROBYTES: usize = crypto_box_BOXZEROBYTES + crypto_box_MACBYTES;
/* #define crypto_box_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX - MACBYTES) */
const crypto_box_MESSAGEBYTES_MAX: u64 = SODIUM_SIZE_MAX - crypto_box_MACBYTES as u64;
/* #define crypto_box_PRIMITIVE "curve25519xsalsa20poly1305" */
static crypto_box_PRIMITIVE: [u8; 27] = *b"curve25519xsalsa20poly1305\0";

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_box/curve25519xsalsa20poly1305/box_curve25519xsalsa20poly1305.c */
    fn crypto_box_curve25519xsalsa20poly1305_seed_keypair(
        pk: *mut u8,
        sk: *mut u8,
        seed: *const u8,
    ) -> c_int;
    fn crypto_box_curve25519xsalsa20poly1305_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn crypto_box_curve25519xsalsa20poly1305_beforenm(
        k: *mut u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
    fn crypto_box_curve25519xsalsa20poly1305_afternm(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_box_curve25519xsalsa20poly1305_open_afternm(
        m: *mut u8,
        c: *const u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_box_curve25519xsalsa20poly1305(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
    fn crypto_box_curve25519xsalsa20poly1305_open(
        m: *mut u8,
        c: *const u8,
        clen: c_ulonglong,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
}

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seedbytes() -> usize {
    crypto_box_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_publickeybytes() -> usize {
    crypto_box_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_secretkeybytes() -> usize {
    crypto_box_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_beforenmbytes() -> usize {
    crypto_box_BEFORENMBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_noncebytes() -> usize {
    crypto_box_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_zerobytes() -> usize {
    crypto_box_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_boxzerobytes() -> usize {
    crypto_box_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_macbytes() -> usize {
    crypto_box_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_messagebytes_max() -> usize {
    crypto_box_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_primitive() -> *const c_char {
    crypto_box_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_beforenm(k, pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_afternm(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_afternm(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_open_afternm(m, c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305(c, m, mlen, n, pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    crypto_box_curve25519xsalsa20poly1305_open(m, c, clen, n, pk, sk)
}
