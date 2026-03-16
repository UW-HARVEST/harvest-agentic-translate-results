#![allow(clippy::missing_safety_doc, non_upper_case_globals)]

mod params;
mod context;
mod address;
mod blake;
mod utils;
mod hash;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod merkle;
mod rng;
mod sign;

use std::slice;

// --- crypto_sign API ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    params::SPX_SK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    params::SPX_PK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    params::SPX_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    params::CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    let pk = unsafe { slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES) };
    let seed = unsafe { slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk = unsafe { slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sig = unsafe { slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let sk = unsafe { slice::from_raw_parts(sk, params::SPX_SK_BYTES) };
    let siglen = unsafe { &mut *siglen };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let sig = unsafe { slice::from_raw_parts(sig, siglen) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let pk = unsafe { slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    let sm = unsafe { slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { slice::from_raw_parts(sk, params::SPX_SK_BYTES) };
    let smlen = unsafe { &mut *smlen };
    sign::crypto_sign(sm, smlen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    let sm = unsafe { slice::from_raw_parts(sm, smlen as usize) };
    let m = unsafe { slice::from_raw_parts_mut(m, smlen as usize) };
    let mlen = unsafe { &mut *mlen };
    let pk = unsafe { slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    sign::crypto_sign_open(m, mlen, sm, smlen, pk)
}

// --- RNG API ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    let ei = unsafe { slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(personalization_string, 48) as &[u8] })
    };
    rng::randombytes_init(ei, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let x = unsafe { slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(x, xlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key = unsafe { slice::from_raw_parts_mut(key, 32) };
    let v = unsafe { slice::from_raw_parts_mut(v, 16) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(provided_data, 48) as &[u8] })
    };
    rng::aes256_ctr_drbg_update(pd, key, v);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx, seed, diversifier, maxlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut u8,
    xlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    let x = unsafe { slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx, x, xlen)
}
