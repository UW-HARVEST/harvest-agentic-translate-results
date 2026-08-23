//! Translation of `crypto_sign/crypto_sign.c`.

use crate::common::*;
use core::ffi::{c_char, c_int, c_ulonglong};

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
 * } crypto_sign_ed25519ph_state;
 * typedef crypto_sign_ed25519ph_state crypto_sign_state;   */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct crypto_sign_ed25519ph_state {
    pub hs: crypto_hash_sha512_state,
}

pub type crypto_sign_state = crypto_sign_ed25519ph_state;

/* #define crypto_sign_BYTES crypto_sign_ed25519_BYTES  (64U) */
const crypto_sign_BYTES: usize = 64;
/* #define crypto_sign_SEEDBYTES crypto_sign_ed25519_SEEDBYTES  (32U) */
const crypto_sign_SEEDBYTES: usize = 32;
/* #define crypto_sign_PUBLICKEYBYTES crypto_sign_ed25519_PUBLICKEYBYTES  (32U) */
const crypto_sign_PUBLICKEYBYTES: usize = 32;
/* #define crypto_sign_SECRETKEYBYTES crypto_sign_ed25519_SECRETKEYBYTES  (32U + 32U) */
const crypto_sign_SECRETKEYBYTES: usize = 32 + 32;
/* #define crypto_sign_MESSAGEBYTES_MAX crypto_sign_ed25519_MESSAGEBYTES_MAX
 *     == (SODIUM_SIZE_MAX - crypto_sign_ed25519_BYTES) */
const crypto_sign_MESSAGEBYTES_MAX: usize = (SODIUM_SIZE_MAX - crypto_sign_BYTES as u64) as usize;
/* #define crypto_sign_PRIMITIVE "ed25519" */
static crypto_sign_PRIMITIVE: [u8; 8] = *b"ed25519\0";

extern "C" {
    fn crypto_sign_ed25519_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn crypto_sign_ed25519_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn crypto_sign_ed25519(
        sm: *mut u8,
        smlen_p: *mut c_ulonglong,
        m: *const u8,
        mlen: c_ulonglong,
        sk: *const u8,
    ) -> c_int;
    fn crypto_sign_ed25519_open(
        m: *mut u8,
        mlen_p: *mut c_ulonglong,
        sm: *const u8,
        smlen: c_ulonglong,
        pk: *const u8,
    ) -> c_int;
    fn crypto_sign_ed25519_detached(
        sig: *mut u8,
        siglen_p: *mut c_ulonglong,
        m: *const u8,
        mlen: c_ulonglong,
        sk: *const u8,
    ) -> c_int;
    fn crypto_sign_ed25519_verify_detached(
        sig: *const u8,
        m: *const u8,
        mlen: c_ulonglong,
        pk: *const u8,
    ) -> c_int;
    fn crypto_sign_ed25519ph_init(state: *mut crypto_sign_ed25519ph_state) -> c_int;
    fn crypto_sign_ed25519ph_update(
        state: *mut crypto_sign_ed25519ph_state,
        m: *const u8,
        mlen: c_ulonglong,
    ) -> c_int;
    fn crypto_sign_ed25519ph_final_create(
        state: *mut crypto_sign_ed25519ph_state,
        sig: *mut u8,
        siglen_p: *mut c_ulonglong,
        sk: *const u8,
    ) -> c_int;
    fn crypto_sign_ed25519ph_final_verify(
        state: *mut crypto_sign_ed25519ph_state,
        sig: *const u8,
        pk: *const u8,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_bytes() -> usize {
    crypto_sign_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seedbytes() -> usize {
    crypto_sign_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_publickeybytes() -> usize {
    crypto_sign_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_secretkeybytes() -> usize {
    crypto_sign_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_messagebytes_max() -> usize {
    crypto_sign_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_primitive() -> *const c_char {
    crypto_sign_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_sign_ed25519_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_sign_ed25519_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519(sm, smlen_p, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen_p: *mut c_ulonglong,
    sm: *const u8,
    smlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_open(m, mlen_p, sm, smlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_detached(
    sig: *mut u8,
    siglen_p: *mut c_ulonglong,
    m: *const u8,
    mlen: c_ulonglong,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: c_ulonglong,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_verify_detached(sig, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_init(state: *mut crypto_sign_state) -> c_int {
    crypto_sign_ed25519ph_init(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_update(
    state: *mut crypto_sign_state,
    m: *const u8,
    mlen: c_ulonglong,
) -> c_int {
    crypto_sign_ed25519ph_update(state, m, mlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_create(
    state: *mut crypto_sign_state,
    sig: *mut u8,
    siglen_p: *mut c_ulonglong,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519ph_final_create(state, sig, siglen_p, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_verify(
    state: *mut crypto_sign_state,
    sig: *const u8,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519ph_final_verify(state, sig, pk)
}
