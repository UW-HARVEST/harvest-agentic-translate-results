//! Translation of `crypto_sign/ed25519/sign_ed25519.c`.

use crate::common::*;
use core::ffi::{c_int, c_ulonglong};

/* Layout of `crypto_hash_sha512_state` from
 * include/sodium/crypto_hash_sha512.h (sizeof == 208 on x86-64). */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

/* typedef struct crypto_sign_ed25519ph_state {
 *     crypto_hash_sha512_state hs;
 * } crypto_sign_ed25519ph_state;                */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_sign_ed25519ph_state {
    pub hs: crypto_hash_sha512_state,
}

/* #define crypto_hash_sha512_BYTES 64U */
const crypto_hash_sha512_BYTES: usize = 64;
/* #define crypto_sign_ed25519_BYTES 64U */
const crypto_sign_ed25519_BYTES: usize = 64;
/* #define crypto_sign_ed25519_SEEDBYTES 32U */
const crypto_sign_ed25519_SEEDBYTES: usize = 32;
/* #define crypto_sign_ed25519_PUBLICKEYBYTES 32U */
const crypto_sign_ed25519_PUBLICKEYBYTES: usize = 32;
/* #define crypto_sign_ed25519_SECRETKEYBYTES (32U + 32U) */
const crypto_sign_ed25519_SECRETKEYBYTES: usize = 32 + 32;
/* #define crypto_sign_ed25519_MESSAGEBYTES_MAX
 *     (SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES) */
const crypto_sign_ed25519_MESSAGEBYTES_MAX: usize =
    (SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES as u64) as usize;

extern "C" {
    fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int;
    fn crypto_hash_sha512_update(
        state: *mut crypto_hash_sha512_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_hash_sha512_final(state: *mut crypto_hash_sha512_state, out: *mut u8) -> c_int;

    /* crypto_sign/ed25519/ref10/sign.c */
    fn _crypto_sign_ed25519_detached(
        sig: *mut u8,
        siglen_p: *mut c_ulonglong,
        m: *const u8,
        mlen: c_ulonglong,
        sk: *const u8,
        prehashed: c_int,
    ) -> c_int;

    /* crypto_sign/ed25519/ref10/open.c */
    fn _crypto_sign_ed25519_verify_detached(
        sig: *const u8,
        m: *const u8,
        mlen: c_ulonglong,
        pk: *const u8,
        prehashed: c_int,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_ed25519ph_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_bytes() -> usize {
    crypto_sign_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_seedbytes() -> usize {
    crypto_sign_ed25519_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_publickeybytes() -> usize {
    crypto_sign_ed25519_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_secretkeybytes() -> usize {
    crypto_sign_ed25519_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_messagebytes_max() -> usize {
    crypto_sign_ed25519_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_seed(
    seed: *mut u8,
    sk: *const u8,
) -> c_int {
    memmove(seed, sk, crypto_sign_ed25519_SEEDBYTES);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519_sk_to_pk(pk: *mut u8, sk: *const u8) -> c_int {
    memmove(
        pk,
        sk.add(crypto_sign_ed25519_SEEDBYTES),
        crypto_sign_ed25519_PUBLICKEYBYTES,
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_init(
    state: *mut crypto_sign_ed25519ph_state,
) -> c_int {
    crypto_hash_sha512_init(&mut (*state).hs);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_update(
    state: *mut crypto_sign_ed25519ph_state,
    m: *const u8,
    mlen: c_ulonglong,
) -> c_int {
    crypto_hash_sha512_update(&mut (*state).hs, m, mlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_create(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *mut u8,
    siglen_p: *mut c_ulonglong,
    sk: *const u8,
) -> c_int {
    let mut ph: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());

    _crypto_sign_ed25519_detached(
        sig,
        siglen_p,
        ph.as_ptr(),
        core::mem::size_of_val(&ph) as c_ulonglong,
        sk,
        1,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_ed25519ph_final_verify(
    state: *mut crypto_sign_ed25519ph_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    let mut ph: [u8; crypto_hash_sha512_BYTES] = [0; crypto_hash_sha512_BYTES];

    crypto_hash_sha512_final(&mut (*state).hs, ph.as_mut_ptr());

    _crypto_sign_ed25519_verify_detached(
        sig,
        ph.as_ptr(),
        core::mem::size_of_val(&ph) as c_ulonglong,
        pk,
        1,
    )
}
