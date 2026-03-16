#![allow(clippy::missing_safety_doc, non_snake_case)]

mod params;
mod context;
mod address;
mod blake256;
mod blake512;
mod hash_blake;
mod thash;
mod utils;
mod utilsx1;
mod wots;
mod wotsx1;
mod fors;
mod merkle;
mod sign;
mod rng;
mod randombytes;

use std::ffi::c_uchar;

// --- Public C API ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    sign::crypto_sign_secretkeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    sign::crypto_sign_publickeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    sign::crypto_sign_bytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    sign::crypto_sign_seedbytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar, sk: *mut c_uchar, seed: *const c_uchar,
) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> i32 {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES) };
    sign::crypto_sign_keypair_impl(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize, m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::SPX_SK_BYTES) };
    let siglen = unsafe { &mut *siglen };
    sign::crypto_sign_signature_impl(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize, m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    sign::crypto_sign_verify_impl(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut c_uchar, smlen: *mut u64,
    m: *const c_uchar, mlen: u64, sk: *const c_uchar,
) -> i32 {
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::SPX_SK_BYTES) };
    let smlen = unsafe { &mut *smlen };
    sign::crypto_sign_impl(sm, smlen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut c_uchar, mlen: *mut u64,
    sm: *const c_uchar, smlen: u64, pk: *const c_uchar,
) -> i32 {
    let m_out = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::SPX_PK_BYTES) };
    let mlen = unsafe { &mut *mlen };
    sign::crypto_sign_open_impl(m_out, mlen, sm, smlen, pk)
}

// rng.c exports
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar, personalization_string: *mut c_uchar,
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
pub extern "C" fn randombytes(x: *mut c_uchar, xlen: u64) -> i32 {
    let x = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::rng_randombytes(x, xlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar, key: *mut c_uchar, v: *mut c_uchar,
) {
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };
    let key = unsafe { &mut *(key as *mut [u8; 32]) };
    let v = unsafe { &mut *(v as *mut [u8; 16]) };
    rng::aes256_ctr_drbg_update(pd, key, v);
}
