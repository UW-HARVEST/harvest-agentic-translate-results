#![allow(clippy::missing_safety_doc)]

pub mod address;
pub mod blake256;
pub mod blake512;
pub mod context;
pub mod fors;
pub mod hash_blake;
pub mod merkle;
pub mod params;
pub mod rng;
pub mod sign;
pub mod thash;
pub mod utils;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

use std::os::raw::{c_int, c_uchar, c_ulonglong};

// ---- crypto_sign API ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    sign::crypto_sign_secretkeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    sign::crypto_sign_publickeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    sign::crypto_sign_bytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    sign::crypto_sign_seedbytes()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
    seed: *const c_uchar,
) -> c_int {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
) -> c_int {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk_s, sk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sig, params::CRYPTO_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let siglen_r = unsafe { &mut *siglen };
    sign::crypto_sign_signature(sig_s, siglen_r, m_s, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let sig_s = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig_s, siglen, m_s, mlen, pk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut c_uchar,
    smlen: *mut c_ulonglong,
    m: *const c_uchar,
    mlen: c_ulonglong,
    sk: *const c_uchar,
) -> c_int {
    let sm_s = unsafe {
        std::slice::from_raw_parts_mut(sm, params::CRYPTO_BYTES + mlen as usize)
    };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let smlen_r = unsafe { &mut *smlen };
    sign::crypto_sign(sm_s, smlen_r, m_s, mlen, sk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut c_ulonglong,
    sm: *const c_uchar,
    smlen: c_ulonglong,
    pk: *const c_uchar,
) -> c_int {
    let m_s = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm_s = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mlen_r = unsafe { &mut *mlen };
    sign::crypto_sign_open(m_s, mlen_r, sm_s, smlen, pk_s)
}

// ---- RNG API ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar,
    personalization_string: *mut c_uchar,
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
pub unsafe extern "C" fn randombytes(
    x: *mut c_uchar,
    xlen: c_ulonglong,
) -> c_int {
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(x_s, xlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar,
    key: *mut c_uchar,
    v: *mut c_uchar,
) {
    let key_s = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v_s = unsafe { std::slice::from_raw_parts_mut(v, 16) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) as &[u8] })
    };
    rng::aes256_ctr_drbg_update(pd, key_s, v_s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut c_uchar,
    diversifier: *mut c_uchar,
    maxlen: u64,
) -> c_int {
    let ctx_r = unsafe { &mut *ctx };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, 32) };
    let div_s = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx_r, seed_s, div_s, maxlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut c_uchar,
    xlen: u64,
) -> c_int {
    let ctx_r = unsafe { &mut *ctx };
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx_r, x_s, xlen as usize)
}
