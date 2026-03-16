#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

mod params;
mod context;
mod address;
mod blake256;
mod blake512;
mod hash_blake;
mod thash;
mod utils;
mod wots;
mod fors;
mod merkle;
mod utilsx1;
mod randombytes;
mod rng;
mod sign;

use std::ffi::c_uchar;

// ---- Public C API ----

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
    pk: *mut c_uchar,
    sk: *mut c_uchar,
    seed: *const c_uchar,
) -> i32 {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
) -> i32 {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair_impl(pk_s, sk_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let siglen_r = unsafe { &mut *siglen };
    sign::crypto_sign_signature_impl(sig_s, siglen_r, m_s, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let sig_s = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify_impl(sig_s, siglen, m_s, mlen, pk_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut c_uchar,
    smlen: *mut u64,
    m: *const c_uchar,
    mlen: u64,
    sk: *const c_uchar,
) -> i32 {
    let sm_s = unsafe { std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let smlen_r = unsafe { &mut *smlen };
    sign::crypto_sign_impl(sm_s, smlen_r, m_s, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut u64,
    sm: *const c_uchar,
    smlen: u64,
    pk: *const c_uchar,
) -> i32 {
    let m_s = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm_s = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mlen_r = unsafe { &mut *mlen };
    sign::crypto_sign_open_impl(m_s, mlen_r, sm_s, smlen, pk_s)
}

// rng.c exports
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar,
    personalization_string: *mut c_uchar,
) {
    let ei = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init_impl(ei, ps);
}

// rng.c also exports randombytes - but we use the /dev/urandom version
// The C library links either randombytes.c OR rng.c, not both.
// We export both randombytes_init (from rng) and randombytes (from randombytes.c)
#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut c_uchar, xlen: u64) {
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    randombytes::randombytes(x_s, xlen);
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_hash_function(ctx: *mut context::SpxCtx) {
    let ctx_r = unsafe { &mut *ctx };
    hash_blake::initialize_hash_function(ctx_r);
}
