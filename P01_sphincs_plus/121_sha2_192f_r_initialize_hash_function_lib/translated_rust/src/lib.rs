#![allow(clippy::missing_safety_doc)]

mod address;
mod context;
mod fors;
mod hash;
mod merkle;
mod params;
mod rng;
mod sha2;
mod sign;
mod thash;
mod utils;
mod wots;

use std::ffi::c_int;
use std::os::raw::{c_uchar, c_ulonglong};

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
    let pk_slice = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_slice = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed_slice = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk_slice, sk_slice, seed_slice)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    let pk_slice = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_slice = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk_slice, sk_slice)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig_slice = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_slice = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let mut sl: usize = 0;
    let ret = sign::crypto_sign_signature(sig_slice, &mut sl, m_slice, mlen, sk_slice);
    unsafe { *siglen = sl };
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let sig_slice = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk_slice = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig_slice, siglen, m_slice, mlen, pk_slice)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut c_uchar,
    smlen: *mut c_ulonglong,
    m: *const c_uchar,
    mlen: c_ulonglong,
    sk: *const c_uchar,
) -> c_int {
    let mlen_usize = mlen as usize;
    let sm_slice =
        unsafe { std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen_usize) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen_usize) };
    let sk_slice = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let mut sl: u64 = 0;
    let ret = sign::crypto_sign_internal(sm_slice, &mut sl, m_slice, mlen, sk_slice);
    unsafe { *smlen = sl };
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut c_ulonglong,
    sm: *const c_uchar,
    smlen: c_ulonglong,
    pk: *const c_uchar,
) -> c_int {
    let smlen_usize = smlen as usize;
    let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
    let sm_slice = unsafe { std::slice::from_raw_parts(sm, smlen_usize) };
    let pk_slice = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mut ml: u64 = 0;
    let ret = sign::crypto_sign_open_internal(m_slice, &mut ml, sm_slice, smlen, pk_slice);
    unsafe { *mlen = ml };
    ret
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
    rng::randombytes_init_internal(ei, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut c_uchar, xlen: c_ulonglong) -> c_int {
    let x_slice = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes_internal(x_slice, xlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar,
    key: *mut c_uchar,
    v: *mut c_uchar,
) {
    let key_slice = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v_slice = unsafe { std::slice::from_raw_parts_mut(v, 16) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) as &[u8] })
    };
    rng::aes256_ctr_drbg_update(pd, key_slice, v_slice);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut c_uchar,
    diversifier: *mut c_uchar,
    maxlen: std::os::raw::c_ulong,
) -> c_int {
    let ctx_ref = unsafe { &mut *ctx };
    let seed_slice = unsafe { std::slice::from_raw_parts(seed, 32) };
    let div_slice = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx_ref, seed_slice, div_slice, maxlen as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut c_uchar,
    xlen: std::os::raw::c_ulong,
) -> c_int {
    let ctx_ref = unsafe { &mut *ctx };
    let x_slice = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx_ref, x_slice, xlen as u64)
}
