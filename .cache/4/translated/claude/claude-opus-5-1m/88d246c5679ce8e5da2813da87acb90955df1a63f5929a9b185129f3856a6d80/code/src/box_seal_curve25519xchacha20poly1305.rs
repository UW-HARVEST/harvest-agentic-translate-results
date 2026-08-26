//! Translation of
//! `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_box_curve25519xchacha20poly1305.h    */
/* ------------------------------------------------------------------------- */

/* #define crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES 32U */
const crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES: usize = 32;
/* #define crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES 32U */
const crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES: usize = 32;
/* #define crypto_box_curve25519xchacha20poly1305_NONCEBYTES 24U */
const crypto_box_curve25519xchacha20poly1305_NONCEBYTES: usize = 24;
/* #define crypto_box_curve25519xchacha20poly1305_MACBYTES 16U */
const crypto_box_curve25519xchacha20poly1305_MACBYTES: usize = 16;
/* #define crypto_box_curve25519xchacha20poly1305_SEALBYTES
 *     (PUBLICKEYBYTES + MACBYTES) */
const crypto_box_curve25519xchacha20poly1305_SEALBYTES: usize =
    crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES
        + crypto_box_curve25519xchacha20poly1305_MACBYTES;
/* #define crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX
 *     (crypto_stream_xchacha20_MESSAGEBYTES_MAX - MACBYTES)
 * with crypto_stream_xchacha20_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX */
const crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64;

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

    /* crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c */
    fn crypto_box_curve25519xchacha20poly1305_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn crypto_box_curve25519xchacha20poly1305_easy(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        pk: *const u8,
        sk: *const u8,
    ) -> c_int;
    fn crypto_box_curve25519xchacha20poly1305_open_easy(
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

unsafe fn _crypto_box_curve25519xchacha20poly1305_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> c_int {
    let mut st = core::mem::MaybeUninit::<crypto_generichash_state>::uninit();
    let st_p = st.as_mut_ptr();

    crypto_generichash_init(
        st_p,
        core::ptr::null(),
        0usize,
        crypto_box_curve25519xchacha20poly1305_NONCEBYTES,
    );
    crypto_generichash_update(
        st_p,
        pk1,
        crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES as c_ulonglong,
    );
    crypto_generichash_update(
        st_p,
        pk2,
        crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES as c_ulonglong,
    );
    crypto_generichash_final(
        st_p,
        nonce,
        crypto_box_curve25519xchacha20poly1305_NONCEBYTES,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    let mut nonce = [0u8; crypto_box_curve25519xchacha20poly1305_NONCEBYTES];
    let mut epk = [0u8; crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES];
    let mut esk = [0u8; crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES];
    let ret: c_int;

    if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_box_curve25519xchacha20poly1305_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(
        nonce.as_mut_ptr(),
        epk.as_ptr(),
        pk,
    );
    ret = crypto_box_curve25519xchacha20poly1305_easy(
        c.add(crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES),
        m,
        mlen,
        nonce.as_ptr(),
        pk,
        esk.as_ptr(),
    );
    memcpy(
        c,
        epk.as_ptr(),
        crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES,
    );
    sodium_memzero(
        esk.as_mut_ptr() as *mut c_void,
        crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut nonce = [0u8; crypto_box_curve25519xchacha20poly1305_NONCEBYTES];

    if clen < crypto_box_curve25519xchacha20poly1305_SEALBYTES as u64 {
        return -1;
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(nonce.as_mut_ptr(), c, pk);

    crypto_box_curve25519xchacha20poly1305_open_easy(
        m,
        c.add(crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES),
        clen - crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES as u64,
        nonce.as_ptr(),
        c,
        sk,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_sealbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SEALBYTES
}
