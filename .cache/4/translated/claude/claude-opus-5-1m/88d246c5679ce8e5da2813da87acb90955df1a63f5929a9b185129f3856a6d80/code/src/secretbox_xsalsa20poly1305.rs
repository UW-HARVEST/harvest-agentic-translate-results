//! Translation of
//! `crypto_secretbox/xsalsa20poly1305/secretbox_xsalsa20poly1305.c`.
//!
//! The reference build defines neither `HAVE_GCC_MEMORY_FENCES` nor
//! `HAVE_C11_MEMORY_FENCES` (no `config.h`), so `ACQUIRE_FENCE` from
//! `private/common.h` expands to `(void) 0`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_secretbox_xsalsa20poly1305.h         */
/* ------------------------------------------------------------------------- */

/* #define crypto_secretbox_xsalsa20poly1305_KEYBYTES 32U */
const crypto_secretbox_xsalsa20poly1305_KEYBYTES: usize = 32;
/* #define crypto_secretbox_xsalsa20poly1305_NONCEBYTES 24U */
const crypto_secretbox_xsalsa20poly1305_NONCEBYTES: usize = 24;
/* #define crypto_secretbox_xsalsa20poly1305_MACBYTES 16U */
const crypto_secretbox_xsalsa20poly1305_MACBYTES: usize = 16;
/* #define crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES 16U */
const crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES: usize = 16;
/* #define crypto_secretbox_xsalsa20poly1305_ZEROBYTES (BOXZEROBYTES + MACBYTES) */
const crypto_secretbox_xsalsa20poly1305_ZEROBYTES: usize =
    crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES + crypto_secretbox_xsalsa20poly1305_MACBYTES;
/* #define crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX
 *     (crypto_stream_xsalsa20_MESSAGEBYTES_MAX - MACBYTES)
 * with crypto_stream_xsalsa20_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX */
const crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_secretbox_xsalsa20poly1305_MACBYTES as u64;

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_stream/xsalsa20/stream_xsalsa20.c */
    fn crypto_stream_xsalsa20(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    /* crypto_onetimeauth/poly1305/onetimeauth_poly1305.c */
    fn crypto_onetimeauth_poly1305(
        out: *mut u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int;
    fn crypto_onetimeauth_poly1305_verify(
        h: *const u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut i: c_int;

    if mlen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k);
    crypto_onetimeauth_poly1305(c.add(16), c.add(32), mlen - 32, c);
    i = 0;
    while i < 16 {
        *c.add(i as usize) = 0;
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_open(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    let mut subkey = [0u8; 32];
    let mut i: c_int;

    if clen < 32 {
        return -1;
    }
    crypto_stream_xsalsa20(subkey.as_mut_ptr(), 32, n, k);
    if crypto_onetimeauth_poly1305_verify(c.add(16), c.add(32), clen - 32, subkey.as_ptr()) != 0 {
        return -1;
    }
    /* ACQUIRE_FENCE; -> (void) 0 in the reference build */
    crypto_stream_xsalsa20_xor(m, c, clen, n, k);
    i = 0;
    while i < 32 {
        *m.add(i as usize) = 0;
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keybytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_noncebytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_zerobytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_ZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_boxzerobytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_BOXZEROBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_macbytes() -> usize {
    crypto_secretbox_xsalsa20poly1305_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_messagebytes_max() -> usize {
    crypto_secretbox_xsalsa20poly1305_MESSAGEBYTES_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_secretbox_xsalsa20poly1305_keygen(k: *mut u8) {
    randombytes_buf(
        k as *mut c_void,
        crypto_secretbox_xsalsa20poly1305_KEYBYTES,
    );
}
