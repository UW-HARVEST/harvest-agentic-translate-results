//! Translation of `crypto_box/crypto_box_seal.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_box.h                                */
/* ------------------------------------------------------------------------- */

/* #define crypto_box_PUBLICKEYBYTES ... (32U) */
const crypto_box_PUBLICKEYBYTES: usize = 32;
/* #define crypto_box_SECRETKEYBYTES ... (32U) */
const crypto_box_SECRETKEYBYTES: usize = 32;
/* #define crypto_box_NONCEBYTES ... (24U) */
const crypto_box_NONCEBYTES: usize = 24;
/* #define crypto_box_MACBYTES ... (16U) */
const crypto_box_MACBYTES: usize = 16;
/* #define crypto_box_SEALBYTES (crypto_box_PUBLICKEYBYTES + crypto_box_MACBYTES) */
const crypto_box_SEALBYTES: usize = crypto_box_PUBLICKEYBYTES + crypto_box_MACBYTES;
/* #define crypto_box_MESSAGEBYTES_MAX (SODIUM_SIZE_MAX - MACBYTES) */
const crypto_box_MESSAGEBYTES_MAX: u64 = SODIUM_SIZE_MAX - crypto_box_MACBYTES as u64;

/// `typedef crypto_generichash_blake2b_state crypto_generichash_state;`
/// (`sizeof` == 384, `_Alignof` == 64).
#[repr(C, align(64))]
struct crypto_generichash_state {
    opaque: [u8; 384],
}

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_generichash/crypto_generichash.c */
    fn crypto_generichash_init(
        state: *mut crypto_generichash_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_update(
        state: *mut crypto_generichash_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_generichash_final(
        state: *mut crypto_generichash_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    /* crypto_box/crypto_box.c */
    fn crypto_box_keypair(pk: *mut u8, sk: *mut u8) -> c_int;

    /* crypto_box/crypto_box_easy.c */
    fn crypto_box_easy(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
    fn crypto_box_open_easy(
        m: *mut u8,
        c: *const u8,
        clen: c_ulonglong,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

/* ------------------------------------------------------------------------- */

unsafe fn _crypto_box_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<crypto_generichash_state>::uninit();
    let st_p = st.as_mut_ptr();

    crypto_generichash_init(st_p, core::ptr::null(), 0usize, crypto_box_NONCEBYTES);
    crypto_generichash_update(st_p, pk1, crypto_box_PUBLICKEYBYTES as c_ulonglong);
    crypto_generichash_update(st_p, pk2, crypto_box_PUBLICKEYBYTES as c_ulonglong);
    crypto_generichash_final(st_p, nonce, crypto_box_NONCEBYTES);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seal(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    let mut nonce = [0u8; crypto_box_NONCEBYTES];
    let mut epk = [0u8; crypto_box_PUBLICKEYBYTES];
    let mut esk = [0u8; crypto_box_SECRETKEYBYTES];
    let ret: c_int;

    if mlen > crypto_box_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_box_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
    ret = crypto_box_easy(
        c.add(crypto_box_PUBLICKEYBYTES),
        m,
        mlen,
        nonce.as_ptr(),
        pk,
        esk.as_ptr(),
    );
    memcpy(c, epk.as_ptr(), crypto_box_PUBLICKEYBYTES);
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, crypto_box_SECRETKEYBYTES);

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut nonce = [0u8; crypto_box_NONCEBYTES];

    if clen < crypto_box_SEALBYTES as u64 {
        return -1;
    }
    _crypto_box_seal_nonce(nonce.as_mut_ptr(), c, pk);

    crypto_box_open_easy(
        m,
        c.add(crypto_box_PUBLICKEYBYTES),
        clen - crypto_box_PUBLICKEYBYTES as u64,
        nonce.as_ptr(),
        c,
        sk,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_sealbytes() -> usize {
    crypto_box_SEALBYTES
}
