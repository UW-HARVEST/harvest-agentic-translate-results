#![allow(clippy::all)]
#![allow(unused_unsafe)]

pub mod params;
pub mod context;
pub mod utils;
pub mod blake;
pub mod hash;
pub mod thash;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod utilsx1;
pub mod merkle;
pub mod rng;
pub mod sign;

use std::ffi::c_int;

// Re-export all public C functions with no_mangle

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
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk, sk) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, params::CRYPTO_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let siglen = unsafe { &mut *siglen };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> c_int {
    let total = params::CRYPTO_BYTES + mlen as usize;
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, total) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let smlen = unsafe { &mut *smlen };
    sign::crypto_sign(sm, smlen, m, mlen, sk) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> c_int {
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let m = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mlen = unsafe { &mut *mlen };
    sign::crypto_sign_open(m, mlen, sm, smlen, pk) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *const u8, personalization_string: *const u8,
) {
    let entropy = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(entropy, ps);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> c_int {
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(buf, xlen);
    0
}
