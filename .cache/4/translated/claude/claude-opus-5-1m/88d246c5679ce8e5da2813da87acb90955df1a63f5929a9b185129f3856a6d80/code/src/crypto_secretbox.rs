//! Translation of `crypto_secretbox/crypto_secretbox.c`.

use crate::common::*;
use core::ffi::{c_char, c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_secretbox.h                          */
/* ------------------------------------------------------------------------- */

/* #define crypto_secretbox_KEYBYTES crypto_secretbox_xsalsa20poly1305_KEYBYTES (32U) */
const crypto_secretbox_KEYBYTES: usize = 32;
/* #define crypto_secretbox_NONCEBYTES ... (24U) */
const crypto_secretbox_NONCEBYTES: usize = 24;
/* #define crypto_secretbox_MACBYTES ... (16U) */
const crypto_secretbox_MACBYTES: usize = 16;
/* #define crypto_secretbox_BOXZEROBYTES ... (16U) */
const crypto_secretbox_BOXZEROBYTES: usize = 16;
/* #define crypto_secretbox_ZEROBYTES ... (BOXZEROBYTES + MACBYTES) */
const crypto_secretbox_ZEROBYTES: usize =
    crypto_secretbox_BOXZEROBYTES + crypto_secretbox_MACBYTES;
/* #define crypto_secretbox_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX - MACBYTES) */
const crypto_secretbox_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_secretbox_MACBYTES as u64;
/* #define crypto_secretbox_PRIMITIVE "xsalsa20poly1305" */
static crypto_secretbox_PRIMITIVE: [u8; 17] = *b"xsalsa20poly1305\0";

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c */
    fn crypto_secretbox_xsalsa20poly1305(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_xsalsa20poly1305_open(
        m: *mut u8,
        c: *const u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keybytes() -> usize {
    crypto_secretbox_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_noncebytes() -> usize {
    crypto_secretbox_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_zerobytes() -> usize {
    crypto_secretbox_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_boxzerobytes() -> usize {
    crypto_secretbox_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_macbytes() -> usize {
    crypto_secretbox_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_messagebytes_max() -> usize {
    crypto_secretbox_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_primitive() -> *const c_char {
    crypto_secretbox_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_open(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xsalsa20poly1305_open(m, c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_secretbox_KEYBYTES);
}
