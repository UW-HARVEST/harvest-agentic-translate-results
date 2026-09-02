//! Translation of
//! `crypto_box/curve25519xchacha20poly1305/box_curve25519xchacha20poly1305.c`,
//! `crypto_box/curve25519xchacha20poly1305/box_seal_curve25519xchacha20poly1305.c`,
//! and `include/sodium/crypto_box_curve25519xchacha20poly1305.h`.

use core::ffi::{c_int, c_void};

use crate::common::memcpy;
use crate::crypto_core::hchacha20::crypto_core_hchacha20;
use crate::crypto_generichash::{
    crypto_generichash_final, crypto_generichash_init, crypto_generichash_state,
    crypto_generichash_update,
};
use crate::crypto_hash::sha512::crypto_hash_sha512;
use crate::crypto_scalarmult::curve25519::{
    crypto_scalarmult_curve25519, crypto_scalarmult_curve25519_base,
};
use crate::crypto_secretbox::xchacha20poly1305::{
    crypto_secretbox_xchacha20poly1305_detached, crypto_secretbox_xchacha20poly1305_open_detached,
};
use crate::crypto_stream::xchacha20::crypto_stream_xchacha20_MESSAGEBYTES_MAX;
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

/* ---- constants from crypto_box_curve25519xchacha20poly1305.h ---- */

pub const crypto_box_curve25519xchacha20poly1305_SEEDBYTES: usize = 32;
pub const crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES: usize = 32;
pub const crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES: usize = 32;
pub const crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES: usize = 32;
pub const crypto_box_curve25519xchacha20poly1305_NONCEBYTES: usize = 24;
pub const crypto_box_curve25519xchacha20poly1305_MACBYTES: usize = 16;
pub const crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX: usize =
    crypto_stream_xchacha20_MESSAGEBYTES_MAX - crypto_box_curve25519xchacha20poly1305_MACBYTES;
pub const crypto_box_curve25519xchacha20poly1305_SEALBYTES: usize =
    crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES
        + crypto_box_curve25519xchacha20poly1305_MACBYTES;

/* ------------------------------------------------------------------ */
/* box_curve25519xchacha20poly1305.c                                   */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut hash: [u8; 64] = [0; 64];

    crypto_hash_sha512(hash.as_mut_ptr(), seed, 32);
    memcpy(sk, hash.as_ptr(), 32);
    sodium_memzero(hash.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&hash));

    crypto_scalarmult_curve25519_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_keypair(
    pk: *mut u8,
    sk: *mut u8,
) -> c_int {
    randombytes_buf(sk as *mut c_void, 32);

    crypto_scalarmult_curve25519_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenm(
    k: *mut u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    static zero: [u8; 16] = [0; 16];
    let mut s: [u8; 32] = [0; 32];

    if crypto_scalarmult_curve25519(s.as_mut_ptr(), sk, pk) != 0 {
        return -1;
    }
    crypto_core_hchacha20(k, zero.as_ptr(), s.as_ptr(), core::ptr::null());
    sodium_memzero(s.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&s));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_detached_afternm(
    c: *mut u8,
    mac: *mut u8,
    m: *const u8,
    mlen: u64,
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
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k: [u8; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES] =
        [0; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_curve25519xchacha20poly1305_detached_afternm(c, mac, m, mlen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_easy_afternm(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
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
    mlen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
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
    clen: u64,
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
    clen: u64,
    n: *const u8,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut k: [u8; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES] =
        [0; crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES];
    let ret: c_int;

    if crypto_box_curve25519xchacha20poly1305_beforenm(k.as_mut_ptr(), pk, sk) != 0 {
        return -1;
    }
    ret = crypto_box_curve25519xchacha20poly1305_open_detached_afternm(m, c, mac, clen, n, k.as_ptr());
    sodium_memzero(k.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_open_easy_afternm(
    m: *mut u8,
    c: *const u8,
    clen: u64,
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
    clen: u64,
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
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_seedbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_publickeybytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_secretkeybytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_beforenmbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_BEFORENMBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_noncebytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_NONCEBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_macbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_MACBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_messagebytes_max() -> usize {
    crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX
}

/* ------------------------------------------------------------------ */
/* box_seal_curve25519xchacha20poly1305.c                              */
/* ------------------------------------------------------------------ */

unsafe fn _crypto_box_curve25519xchacha20poly1305_seal_nonce(
    nonce: *mut u8,
    pk1: *const u8,
    pk2: *const u8,
) -> c_int {
    let mut st: crypto_generichash_state = core::mem::zeroed();

    crypto_generichash_init(
        &mut st,
        core::ptr::null(),
        0usize,
        crypto_box_curve25519xchacha20poly1305_NONCEBYTES,
    );
    crypto_generichash_update(
        &mut st,
        pk1,
        crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES as u64,
    );
    crypto_generichash_update(
        &mut st,
        pk2,
        crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES as u64,
    );
    crypto_generichash_final(
        &mut st,
        nonce,
        crypto_box_curve25519xchacha20poly1305_NONCEBYTES,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    pk: *const u8,
) -> c_int {
    let mut nonce: [u8; crypto_box_curve25519xchacha20poly1305_NONCEBYTES] =
        [0; crypto_box_curve25519xchacha20poly1305_NONCEBYTES];
    let mut epk: [u8; crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES] =
        [0; crypto_box_curve25519xchacha20poly1305_PUBLICKEYBYTES];
    let mut esk: [u8; crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES] =
        [0; crypto_box_curve25519xchacha20poly1305_SECRETKEYBYTES];
    let ret: c_int;

    if mlen > crypto_box_curve25519xchacha20poly1305_MESSAGEBYTES_MAX as u64 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_box_curve25519xchacha20poly1305_keypair(epk.as_mut_ptr(), esk.as_mut_ptr()) != 0 {
        return -1; /* LCOV_EXCL_LINE */
    }
    _crypto_box_curve25519xchacha20poly1305_seal_nonce(nonce.as_mut_ptr(), epk.as_ptr(), pk);
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
    sodium_memzero(esk.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&esk));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_box_curve25519xchacha20poly1305_seal_open(
    m: *mut u8,
    c: *const u8,
    clen: u64,
    pk: *const u8,
    sk: *const u8,
) -> c_int {
    let mut nonce: [u8; crypto_box_curve25519xchacha20poly1305_NONCEBYTES] =
        [0; crypto_box_curve25519xchacha20poly1305_NONCEBYTES];

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
pub extern "C" fn crypto_box_curve25519xchacha20poly1305_sealbytes() -> usize {
    crypto_box_curve25519xchacha20poly1305_SEALBYTES
}
