#![allow(clippy::missing_safety_doc)]
#![allow(static_mut_refs)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod params;
mod context;
mod address;
mod sha2;
mod hash_sha2;
mod thash;
mod utils;
mod wots;
mod fors;
mod utilsx1;
mod merkle;
mod rng;
mod sign;

use std::ffi::c_int;

// --- Public API ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    params::CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    params::CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    params::CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    params::CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let siglen = unsafe { &mut *siglen };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::SPX_SK_BYTES) };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64, m: *const u8, mlen: u64, sk: *const u8,
) -> c_int {
    let total = params::SPX_BYTES + mlen as usize;
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, total) };
    let smlen = unsafe { &mut *smlen };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::SPX_SK_BYTES) };
    sign::crypto_sign_fn(sm, smlen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64, sm: *const u8, smlen: u64, pk: *const u8,
) -> c_int {
    let sm_slice = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let mlen = unsafe { &mut *mlen };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    sign::crypto_sign_open(m_slice, mlen, sm_slice, smlen, pk)
}

// --- RNG API ---

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8, personalization_string: *mut u8,
) {
    let ei = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(ei, ps);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> c_int {
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(x, xlen as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct, seed: *mut u8, diversifier: *mut u8, maxlen: u64,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { std::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx, seed, diversifier, maxlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct, x: *mut u8, xlen: u64,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx, x, xlen as usize)
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8, key: *mut u8, v: *mut u8,
) {
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };
    let key = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v = unsafe { std::slice::from_raw_parts_mut(v, 16) };
    rng::aes256_ctr_drbg_update(pd, key, v);
}
