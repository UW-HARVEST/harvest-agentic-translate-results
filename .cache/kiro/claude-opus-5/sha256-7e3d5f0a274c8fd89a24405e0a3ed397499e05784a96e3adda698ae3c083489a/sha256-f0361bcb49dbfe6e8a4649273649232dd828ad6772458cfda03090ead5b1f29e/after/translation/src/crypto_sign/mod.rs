pub mod ed25519;

// Translation of `crypto_sign/crypto_sign.c` and
// `include/sodium/crypto_sign.h`.

use core::ffi::{c_char, c_int};

use crate::crypto_sign::ed25519::{
    crypto_sign_ed25519, crypto_sign_ed25519_detached, crypto_sign_ed25519_keypair,
    crypto_sign_ed25519_open, crypto_sign_ed25519_seed_keypair,
    crypto_sign_ed25519_verify_detached, crypto_sign_ed25519ph_final_create,
    crypto_sign_ed25519ph_final_verify, crypto_sign_ed25519ph_init,
    crypto_sign_ed25519ph_update, crypto_sign_ed25519_BYTES,
    crypto_sign_ed25519_MESSAGEBYTES_MAX, crypto_sign_ed25519_PUBLICKEYBYTES,
    crypto_sign_ed25519_SECRETKEYBYTES, crypto_sign_ed25519_SEEDBYTES,
    crypto_sign_ed25519ph_state,
};

/* ---- from include/sodium/crypto_sign.h ---- */

/// `typedef crypto_sign_ed25519ph_state crypto_sign_state;`
pub type crypto_sign_state = crypto_sign_ed25519ph_state;

pub const crypto_sign_BYTES: usize = crypto_sign_ed25519_BYTES;
pub const crypto_sign_SEEDBYTES: usize = crypto_sign_ed25519_SEEDBYTES;
pub const crypto_sign_PUBLICKEYBYTES: usize = crypto_sign_ed25519_PUBLICKEYBYTES;
pub const crypto_sign_SECRETKEYBYTES: usize = crypto_sign_ed25519_SECRETKEYBYTES;
pub const crypto_sign_MESSAGEBYTES_MAX: usize = crypto_sign_ed25519_MESSAGEBYTES_MAX;
pub const crypto_sign_PRIMITIVE: &[u8] = b"ed25519\0";

/* ---- from crypto_sign.c ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_statebytes() -> usize {
    core::mem::size_of::<crypto_sign_state>()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> usize {
    crypto_sign_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> usize {
    crypto_sign_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> usize {
    crypto_sign_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> usize {
    crypto_sign_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_messagebytes_max() -> usize {
    crypto_sign_MESSAGEBYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_primitive() -> *const c_char {
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
    smlen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519(sm, smlen_p, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen_p: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    crypto_sign_ed25519_open(m, mlen_p, sm, smlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_detached(
    sig: *mut u8,
    siglen_p: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    crypto_sign_ed25519_detached(sig, siglen_p, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify_detached(
    sig: *const u8,
    m: *const u8,
    mlen: u64,
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
    mlen: u64,
) -> c_int {
    crypto_sign_ed25519ph_update(state, m, mlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_final_create(
    state: *mut crypto_sign_state,
    sig: *mut u8,
    siglen_p: *mut u64,
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
