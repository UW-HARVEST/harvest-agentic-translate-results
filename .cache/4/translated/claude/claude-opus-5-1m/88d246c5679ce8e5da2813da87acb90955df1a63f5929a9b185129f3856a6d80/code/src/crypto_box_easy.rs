//! Translation of `crypto_box/crypto_box_easy.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_box.h                                */
/* ------------------------------------------------------------------------- */

/* #define crypto_box_BEFORENMBYTES ... (32U) */
const crypto_box_BEFORENMBYTES: usize = 32;
/* #define crypto_box_MACBYTES ... (16U) */
const crypto_box_MACBYTES: usize = 16;
/* #define crypto_box_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX - MACBYTES) */
const crypto_box_MESSAGEBYTES_MAX: u64 = SODIUM_SIZE_MAX - crypto_box_MACBYTES as u64;

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_secretbox/crypto_secretbox_easy.c */
    fn crypto_secretbox_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    /* crypto_box/crypto_box.c */
    fn crypto_box_beforenm(k: *mut u8, pk: *const u8, sk: *const u8) -> c_int;

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_detached(c, mac, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; crypto_box_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, crypto_box_BEFORENMBYTES);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_box_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_box_detached_afternm(c.add(crypto_box_MACBYTES), c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_easy(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if mlen > crypto_box_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_box_detached(c.add(crypto_box_MACBYTES), c, m, mlen, n, pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_open_detached(m, c, mac, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; crypto_box_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, crypto_box_BEFORENMBYTES);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < crypto_box_MACBYTES as u64 {
        return -1;
    }
    crypto_box_open_detached_afternm(
        m,
        c.add(crypto_box_MACBYTES),
        c,
        clen - crypto_box_MACBYTES as u64,
        n,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if clen < crypto_box_MACBYTES as u64 {
        return -1;
    }
    crypto_box_open_detached(
        m,
        c.add(crypto_box_MACBYTES),
        c,
        clen - crypto_box_MACBYTES as u64,
        n,
        pk,
        sk,
    )
}
