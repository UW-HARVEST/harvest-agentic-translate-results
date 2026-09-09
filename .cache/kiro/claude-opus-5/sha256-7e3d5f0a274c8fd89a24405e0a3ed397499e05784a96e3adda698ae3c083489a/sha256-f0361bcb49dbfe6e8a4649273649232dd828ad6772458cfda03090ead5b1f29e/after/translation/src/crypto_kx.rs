//! Translation of `crypto_kx/crypto_kx.c` and `include/sodium/crypto_kx.h`.

use core::ffi::{c_char, c_int, c_void};

use crate::crypto_generichash::{
    crypto_generichash, crypto_generichash_final, crypto_generichash_init,
    crypto_generichash_state, crypto_generichash_update,
};
use crate::crypto_scalarmult::{
    crypto_scalarmult, crypto_scalarmult_base, crypto_scalarmult_BYTES,
};
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

/* ---- constants from crypto_kx.h ---- */

pub const crypto_kx_PUBLICKEYBYTES: usize = 32;
pub const crypto_kx_SECRETKEYBYTES: usize = 32;
pub const crypto_kx_SEEDBYTES: usize = 32;
pub const crypto_kx_SESSIONKEYBYTES: usize = 32;
pub const crypto_kx_PRIMITIVE: &[u8] = b"x25519blake2b\0";

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
        crypto_kx_SEEDBYTES as u64,
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
    mut rx: *mut u8,
    mut tx: *mut u8,
    client_pk: *const u8,
    client_sk: *const u8,
    server_pk: *const u8,
) -> c_int {
    let mut h: crypto_generichash_state = core::mem::zeroed();
    let mut q: [u8; crypto_scalarmult_BYTES] = [0; crypto_scalarmult_BYTES];
    let mut keys: [u8; 2 * crypto_kx_SESSIONKEYBYTES] = [0; 2 * crypto_kx_SESSIONKEYBYTES];
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
    crypto_generichash_init(&mut h, core::ptr::null(), 0usize, core::mem::size_of_val(&keys));
    crypto_generichash_update(&mut h, q.as_ptr(), crypto_scalarmult_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&q));
    crypto_generichash_update(&mut h, client_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut h, server_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), core::mem::size_of_val(&keys));
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
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&keys));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kx_server_session_keys(
    mut rx: *mut u8,
    mut tx: *mut u8,
    server_pk: *const u8,
    server_sk: *const u8,
    client_pk: *const u8,
) -> c_int {
    let mut h: crypto_generichash_state = core::mem::zeroed();
    let mut q: [u8; crypto_scalarmult_BYTES] = [0; crypto_scalarmult_BYTES];
    let mut keys: [u8; 2 * crypto_kx_SESSIONKEYBYTES] = [0; 2 * crypto_kx_SESSIONKEYBYTES];
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
    crypto_generichash_init(&mut h, core::ptr::null(), 0usize, core::mem::size_of_val(&keys));
    crypto_generichash_update(&mut h, q.as_ptr(), crypto_scalarmult_BYTES as u64);
    sodium_memzero(q.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&q));
    crypto_generichash_update(&mut h, client_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_update(&mut h, server_pk, crypto_kx_PUBLICKEYBYTES as u64);
    crypto_generichash_final(&mut h, keys.as_mut_ptr(), core::mem::size_of_val(&keys));
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
    sodium_memzero(keys.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&keys));

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_publickeybytes() -> usize {
    crypto_kx_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_secretkeybytes() -> usize {
    crypto_kx_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_seedbytes() -> usize {
    crypto_kx_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_sessionkeybytes() -> usize {
    crypto_kx_SESSIONKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kx_primitive() -> *const c_char {
    crypto_kx_PRIMITIVE.as_ptr() as *const c_char
}
