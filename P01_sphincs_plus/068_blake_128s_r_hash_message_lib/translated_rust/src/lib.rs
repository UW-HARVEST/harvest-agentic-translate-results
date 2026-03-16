#![allow(clippy::too_many_arguments)]

pub mod params;
pub mod context;
pub mod address;
pub mod utils;
pub mod blake256;
pub mod blake512;
pub mod hash_blake;
pub mod thash;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod forsx1;
pub mod utilsx1;
pub mod merkle;
pub mod sign;
pub mod rng;
pub mod randombytes;

use params::*;

// ---- Public C API ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    CRYPTO_SECRETKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    CRYPTO_PUBLICKEYBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    CRYPTO_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    CRYPTO_SEEDBYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let pk = unsafe { core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk = unsafe { core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig = unsafe { core::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let siglen = unsafe { &mut *siglen };
    let m = unsafe { core::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let sig = unsafe { core::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { core::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let sm = unsafe { core::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let smlen = unsafe { &mut *smlen };
    let m = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign(sm, smlen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let m_out = unsafe { core::slice::from_raw_parts_mut(m, smlen as usize) };
    let mlen = unsafe { &mut *mlen };
    let sm = unsafe { core::slice::from_raw_parts(sm, smlen as usize) };
    let pk = unsafe { core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_open(m_out, mlen, sm, smlen, pk)
}

// RNG API
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    let entropy = unsafe { core::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(entropy, ps);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let buf = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes_rng(buf, xlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key_slice = unsafe { core::slice::from_raw_parts_mut(key, 32) };
    let v_slice = unsafe { core::slice::from_raw_parts_mut(v, 16) };
    let data = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(provided_data, 48) })
    };
    rng::aes256_ctr_drbg_update(data, key_slice, v_slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *const u8,
    diversifier: *const u8,
    maxlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { core::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { core::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx, seed, diversifier, maxlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut u8,
    xlen: u64,
) -> i32 {
    let ctx = unsafe { &mut *ctx };
    let buf = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx, buf, xlen as usize)
}
