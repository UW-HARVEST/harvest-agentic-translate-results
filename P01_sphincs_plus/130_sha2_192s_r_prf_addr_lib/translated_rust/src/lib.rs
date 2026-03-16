#![allow(clippy::missing_safety_doc)]

pub mod params;
pub mod context;
pub mod sha2;
pub mod address;
pub mod utils;
pub mod hash_sha2;
pub mod thash;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod utilsx1;
pub mod merkle;
pub mod sign;

use std::os::raw::{c_int, c_uchar, c_ulonglong};

// --- Public C API ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    params::CRYPTO_SECRETKEYBYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    params::CRYPTO_PUBLICKEYBYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    params::CRYPTO_BYTES as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    params::CRYPTO_SEEDBYTES as c_ulonglong
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
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
) -> c_int {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk_s, sk_s) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let siglen_r = unsafe { &mut *siglen };
    sign::crypto_sign_signature(sig_s, siglen_r, m_s, mlen, sk_s) as c_int
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
    sign::crypto_sign_verify(sig_s, siglen, m_s, mlen, pk_s) as c_int
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
        std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize)
    };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let smlen_r = unsafe { &mut *smlen };
    sign::crypto_sign(sm_s, smlen_r, m_s, mlen, sk_s) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut c_ulonglong,
    sm: *const c_uchar,
    smlen: c_ulonglong,
    pk: *const c_uchar,
) -> c_int {
    let m_s = unsafe {
        std::slice::from_raw_parts_mut(m, smlen as usize)
    };
    let sm_s = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mlen_r = unsafe { &mut *mlen };
    sign::crypto_sign_open(m_s, mlen_r, sm_s, smlen, pk_s) as c_int
}
