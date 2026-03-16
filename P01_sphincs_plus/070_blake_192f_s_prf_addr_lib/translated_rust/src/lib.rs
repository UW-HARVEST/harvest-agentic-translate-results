#![allow(clippy::too_many_arguments)]

mod address;
mod blake256;
mod blake512;
mod context;
mod fors;
mod hash_blake;
mod merkle;
mod params;
mod randombytes;
mod sign;
mod thash;
mod utils;
mod utilsx1;
mod wots;
mod wotsx1;

use std::slice;

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
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    unsafe {
        let pk = slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES);
        let sk = slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES);
        let seed = slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES);
        sign::crypto_sign_seed_keypair(pk, sk, seed)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    unsafe {
        let pk = slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES);
        let sk = slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES);
        sign::crypto_sign_keypair(pk, sk)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sig = slice::from_raw_parts_mut(sig, params::CRYPTO_BYTES);
        let m = slice::from_raw_parts(m, mlen);
        let sk = slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES);
        let mut sl: usize = 0;
        let ret = sign::crypto_sign_signature(sig, &mut sl, m, mlen, sk);
        *siglen = sl;
        ret
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    unsafe {
        let sig = slice::from_raw_parts(sig, siglen);
        let m = slice::from_raw_parts(m, mlen);
        let pk = slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES);
        sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sm = slice::from_raw_parts_mut(sm, params::CRYPTO_BYTES + mlen as usize);
        let m = slice::from_raw_parts(m, mlen as usize);
        let sk = slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES);
        let mut sl: u64 = 0;
        let ret = sign::crypto_sign(sm, &mut sl, m, mlen, sk);
        *smlen = sl;
        ret
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    unsafe {
        let m_out = slice::from_raw_parts_mut(m, smlen as usize);
        let sm = slice::from_raw_parts(sm, smlen as usize);
        let pk = slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES);
        let mut ml: u64 = 0;
        let ret = sign::crypto_sign_open(m_out, &mut ml, sm, smlen, pk);
        *mlen = ml;
        ret
    }
}
