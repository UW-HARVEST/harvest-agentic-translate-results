//! Translation of `c_src/libsodium/crypto_kx/crypto_kx.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::types::crypto_generichash_blake2b_state as crypto_generichash_state;

const crypto_kx_PUBLICKEYBYTES: usize = 32;
const crypto_kx_SECRETKEYBYTES: usize = 32;
const crypto_kx_SEEDBYTES: usize = 32;
const crypto_kx_SESSIONKEYBYTES: usize = 32;
const crypto_kx_PRIMITIVE: &[u8] = b"x25519blake2b\0";

const crypto_scalarmult_BYTES: usize = 32;

extern "C" {
    fn crypto_generichash(
        out: *mut u8,
        outlen: usize,
        inp: *const u8,
        inlen: u64,
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
        inp: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_final(
        state: *mut crypto_generichash_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
    fn crypto_scalarmult(q: *mut u8, n: *const u8, p: *const u8) -> c_int;
    fn crypto_scalarmult_base(q: *mut u8, n: *const u8) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_misuse() -> !;
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_generichash(
        sk,
        crypto_kx_SECRETKEYBYTES,
        seed,
        crypto_kx_SEEDBYTES as u64,
        core::ptr::null(),
        0,
    );
    crypto_scalarmult_base(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    randombytes_buf(sk as *mut c_void, crypto_kx_SECRETKEYBYTES);
    crypto_scalarmult_base(pk, sk)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_client_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    client_pk: *const u8,
    client_sk: *const u8,
    server_pk: *const u8,
) -> c_int {
    let mut h: crypto_generichash_state = crypto_generichash_state { opaque: [0u8; 384] };
    let mut q = [0u8; crypto_scalarmult_BYTES];
    let mut keys = [0u8; 2 * crypto_kx_SESSIONKEYBYTES];

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
    crypto_generichash_update(&mut h, q.as_ptr(), crypto_scalarmult_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, q.len());
    crypto_generichash_update(&mut h, client_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut h, server_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), keys.len());
    sodium_memzero(
        &mut h as *mut crypto_generichash_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_state>(),
    );
    for i in 0..crypto_kx_SESSIONKEYBYTES {
        *rx.add(i) = keys[i]; /* rx cannot be NULL */
        *tx.add(i) = keys[i + crypto_kx_SESSIONKEYBYTES]; /* tx cannot be NULL */
    }
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, keys.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_server_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    server_pk: *const u8,
    server_sk: *const u8,
    client_pk: *const u8,
) -> c_int {
    let mut h: crypto_generichash_state = crypto_generichash_state { opaque: [0u8; 384] };
    let mut q = [0u8; crypto_scalarmult_BYTES];
    let mut keys = [0u8; 2 * crypto_kx_SESSIONKEYBYTES];

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
    crypto_generichash_update(&mut h, q.as_ptr(), crypto_scalarmult_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, q.len());
    crypto_generichash_update(&mut h, client_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut h, server_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), keys.len());
    sodium_memzero(
        &mut h as *mut crypto_generichash_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_state>(),
    );
    for i in 0..crypto_kx_SESSIONKEYBYTES {
        *tx.add(i) = keys[i];
        *rx.add(i) = keys[i + crypto_kx_SESSIONKEYBYTES];
    }
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, keys.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_publickeybytes() -> usize {
    crypto_kx_PUBLICKEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_secretkeybytes() -> usize {
    crypto_kx_SECRETKEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_seedbytes() -> usize {
    crypto_kx_SEEDBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_sessionkeybytes() -> usize {
    crypto_kx_SESSIONKEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_kx_primitive() -> *const c_char {
    crypto_kx_PRIMITIVE.as_ptr() as *const c_char
}
