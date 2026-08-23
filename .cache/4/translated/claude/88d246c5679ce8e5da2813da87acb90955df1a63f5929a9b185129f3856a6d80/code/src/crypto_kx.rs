//! Translation of `crypto_kx/crypto_kx.c`.
//!
//! Exports:
//!   * `crypto_kx_client_session_keys`
//!   * `crypto_kx_keypair`
//!   * `crypto_kx_primitive`
//!   * `crypto_kx_publickeybytes`
//!   * `crypto_kx_secretkeybytes`
//!   * `crypto_kx_seed_keypair`
//!   * `crypto_kx_seedbytes`
//!   * `crypto_kx_server_session_keys`
//!   * `crypto_kx_sessionkeybytes`

use core::ffi::{c_char, c_int, c_ulonglong, c_void};

/* crypto_kx.h */
const crypto_kx_PUBLICKEYBYTES: usize = 32;
const crypto_kx_SECRETKEYBYTES: usize = 32;
const crypto_kx_SEEDBYTES: usize = 32;
const crypto_kx_SESSIONKEYBYTES: usize = 32;
const crypto_kx_PRIMITIVE: &[u8; 14] = b"x25519blake2b\0";

/* crypto_scalarmult.h -> crypto_scalarmult_curve25519.h */
const crypto_scalarmult_BYTES: usize = 32;

/* typedef crypto_generichash_blake2b_state crypto_generichash_state;
 * sizeof == 384, _Alignof == 64. */
#[repr(C, align(64))]
struct crypto_generichash_state {
    opaque: [u8; 384],
}

extern "C" {
    /* crypto_generichash/crypto_generichash.c */
    fn crypto_generichash(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: c_ulonglong,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
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
    /* crypto_scalarmult/crypto_scalarmult.c */
    fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int;
    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);
    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    /* sodium/core.c */
    fn sodium_misuse() -> !;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_generichash(
        sk,
        crypto_kx_SECRETKEYBYTES,
        seed,
        crypto_kx_SEEDBYTES as c_ulonglong,
        core::ptr::null(),
        0,
    );
    crypto_scalarmult_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    randombytes_buf(sk as *mut c_void, crypto_kx_SECRETKEYBYTES);
    crypto_scalarmult_base(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_client_session_keys(
    rx: *mut u8,
    tx: *mut u8,
    client_pk: *const u8,
    client_sk: *const u8,
    server_pk: *const u8,
) -> c_int {
    let mut rx = rx;
    let mut tx = tx;
    let mut h = crypto_generichash_state { opaque: [0u8; 384] };
    let mut q: [u8; crypto_scalarmult_BYTES] = [0u8; crypto_scalarmult_BYTES];
    let mut keys: [u8; 2 * crypto_kx_SESSIONKEYBYTES] = [0u8; 2 * crypto_kx_SESSIONKEYBYTES];
    let mut i: c_int;

    if rx.is_null() {
        rx = tx;
    }
    if tx.is_null() {
        tx = rx;
    }
    if rx.is_null() {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_scalarmult(q.as_mut_ptr(), client_sk, server_pk) != 0 {
        return -1;
    }
    crypto_generichash_init(&mut h, core::ptr::null(), 0, keys.len());
    crypto_generichash_update(&mut h, q.as_ptr(), crypto_scalarmult_BYTES as c_ulonglong);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&q));
    crypto_generichash_update(&mut h, client_pk, crypto_kx_PUBLICKEYBYTES as c_ulonglong);
    crypto_generichash_update(&mut h, server_pk, crypto_kx_PUBLICKEYBYTES as c_ulonglong);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), keys.len());
    sodium_memzero(
        &mut h as *mut crypto_generichash_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_state>(),
    );
    i = 0;
    while i < crypto_kx_SESSIONKEYBYTES as c_int {
        *rx.add(i as usize) = keys[i as usize]; /* rx cannot be NULL */
        *tx.add(i as usize) = keys[i as usize + crypto_kx_SESSIONKEYBYTES]; /* tx cannot be NULL */
        i += 1;
    }
    sodium_memzero(
        keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&keys),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_server_session_keys(
    rx: *mut u8,
    tx: *mut u8,
    server_pk: *const u8,
    server_sk: *const u8,
    client_pk: *const u8,
) -> c_int {
    let mut rx = rx;
    let mut tx = tx;
    let mut h = crypto_generichash_state { opaque: [0u8; 384] };
    let mut q: [u8; crypto_scalarmult_BYTES] = [0u8; crypto_scalarmult_BYTES];
    let mut keys: [u8; 2 * crypto_kx_SESSIONKEYBYTES] = [0u8; 2 * crypto_kx_SESSIONKEYBYTES];
    let mut i: c_int;

    if rx.is_null() {
        rx = tx;
    }
    if tx.is_null() {
        tx = rx;
    }
    if rx.is_null() {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if crypto_scalarmult(q.as_mut_ptr(), server_sk, client_pk) != 0 {
        return -1;
    }
    crypto_generichash_init(&mut h, core::ptr::null(), 0, keys.len());
    crypto_generichash_update(&mut h, q.as_ptr(), crypto_scalarmult_BYTES as c_ulonglong);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&q));
    crypto_generichash_update(&mut h, client_pk, crypto_kx_PUBLICKEYBYTES as c_ulonglong);
    crypto_generichash_update(&mut h, server_pk, crypto_kx_PUBLICKEYBYTES as c_ulonglong);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), keys.len());
    sodium_memzero(
        &mut h as *mut crypto_generichash_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_state>(),
    );
    i = 0;
    while i < crypto_kx_SESSIONKEYBYTES as c_int {
        *tx.add(i as usize) = keys[i as usize];
        *rx.add(i as usize) = keys[i as usize + crypto_kx_SESSIONKEYBYTES];
        i += 1;
    }
    sodium_memzero(
        keys.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&keys),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_publickeybytes() -> usize {
    crypto_kx_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_secretkeybytes() -> usize {
    crypto_kx_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_seedbytes() -> usize {
    crypto_kx_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_sessionkeybytes() -> usize {
    crypto_kx_SESSIONKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_primitive() -> *const c_char {
    crypto_kx_PRIMITIVE.as_ptr() as *const c_char
}
