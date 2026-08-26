//! Translation of
//! `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/* ------------------------------------------------------------------------- */
/* Constants from include/sodium/crypto_box_curve25519xchacha20poly1305.h    */
/* ------------------------------------------------------------------------- */

/* #define crypto_box_curve25519xchacha20poly1305_SEEDBYTES 32U */
const crypto_box_curve25519xchacha20poly1305_SEEDBYTES: usize = 32;
/* #define crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES 32U */
const crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES: usize = 32;
/* #define crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES 32U */
const crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES: usize = 32;
/* #define crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES 32U */
const crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES: usize = 32;
/* #define crypto_box_curve25519xchacha20poly1305_NONCEBYTES 24U */
const crypto_box_curve25519xchacha20poly1305_NONCEBYTES: usize = 24;
/* #define crypto_box_curve25519xchacha20poly1305_MACBYTES 16U */
const crypto_box_curve25519xchacha20poly1305_MACBYTES: usize = 16;
/* #define crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX
 *     (crypto_stream_xchacha20_MESSAGEBYTES_MAX - MACBYTES)
 * with crypto_stream_xchacha20_MESSAGEBYTES_MAX == SODIUM_SIZE_MAX */
const crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX: u64 =
    SODIUM_SIZE_MAX - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64;

/* ------------------------------------------------------------------------- */
/* Cross-file declarations (resolved by the linker inside the cdylib)        */
/* ------------------------------------------------------------------------- */

extern "C" {
    /* crypto_core/hchacha20/core_hchacha20.c */
    fn crypto_core_hchacha20(
        out: *mut u8,
        in_: *const u8,
        k: *const u8,
        c: *const u8,
    ) -> c_int;

    /* crypto_hash/sha512/cp/hash_sha512_cp.c */
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: c_ulonglong) -> c_int;

    /* crypto_scalarmult/curve25519/scalarmult_curve25519.c */
    fn crypto_scalarmult_curve25519(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_curve25519_base(q: *mut u8, n: *const u8) -> c_int;

    /* crypto_secretbox/xchacha20poly1305/secretbox_xchacha20poly1305.c */
    fn crypto_secretbox_xchacha20poly1305_detached(
        c: *mut u8,
        mac: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_secretbox_xchacha20poly1305_open_detached(
        m: *mut u8,
        c: *const u8,
        mac: *const u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

/* `static const unsigned char zero[16] = { 0 };` from
 * crypto_box_curve25519xchacha20poly1305_beforenm() */
static zero: [u8; 16] = [0u8; 16];

/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut hash = [0u8; 64];

    crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
    memcpy(sk, hash.as_ptr(), 32);
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, 64);

    crypto_scalarmult_curve25519_base(pk, sk as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    randombytes_buf(sk as *mut c_void, 32);

    crypto_scalarmult_curve25519_base(pk, sk as *const u8)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut s = [0u8; 32];

    if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
        return -1;
    }
    crypto_core_hchacha20(k, zero.as_ptr(), s.as_ptr(), core::ptr::null());
    sodium_memzero(s.as_mut_ptr() as *mut c_void, 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xchacha20poly1305_detached(c, mac, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_curve25519xchacha20poly1305_detached_afternm(
        c,
        mac,
        m,
        mlen,
        n,
        k.as_ptr(),
    );
    sodium_memzero(
        k.as_mut_ptr() as *mut c_void,
        crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_box_curve25519xchacha20poly1305_detached_afternm(
        c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
        c,
        m,
        mlen,
        n,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    crypto_box_curve25519xchacha20poly1305_detached(
        c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
        c,
        m,
        mlen,
        n,
        pk,
        sk,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_secretbox_xchacha20poly1305_open_detached(m, c, mac, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_detached(
    m: *mut u8,
    c: *const u8,
    mac: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k = [0u8; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
        m,
        c,
        mac,
        clen,
        n,
        k.as_ptr(),
    );
    sodium_memzero(
        k.as_mut_ptr() as *mut c_void,
        crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES,
    );

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if clen < crypto_box_curve25519xchacha20poly1305_MACBYTES as u64 {
        return -1;
    }
    crypto_box_curve25519xchacha20poly1305_open_detached_afternm(
        m,
        c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
        c,
        clen - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64,
        n,
        k,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy(
    m: *mut u8,
    c: *const u8,
    clen: c_ulonglong,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if clen < crypto_box_curve25519xchacha20poly1305_MACBYTES as u64 {
        return -1;
    }
    crypto_box_curve25519xchacha20poly1305_open_detached(
        m,
        c.add(crypto_box_curve25519xchacha20poly1305_MACBYTES),
        c,
        clen - crypto_box_curve25519xchacha20poly1305_MACBYTES as u64,
        n,
        pk,
        sk,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seedbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_publickeybytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_secretkeybytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenmbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_noncebytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_macbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_MACBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_messagebytes_max() -> usize {
    crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX as usize
}
