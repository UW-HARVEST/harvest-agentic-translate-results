#![allow(clippy::missing_safety_doc)]

mod params;
mod context;
mod address;
mod blake256;
mod hash;
mod thash;
mod wots;
mod fors;
mod utils;
mod utilsx1;
mod merkle;
mod rng;
mod randombytes;
mod sign;

use std::ffi::c_int;
use std::os::raw::c_uchar;

// ---- Size query functions ----

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

// ---- Key generation ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar, sk: *mut c_uchar, seed: *const c_uchar,
) -> c_int {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
}

/// Uses /dev/urandom
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(
    pk: *mut c_uchar, sk: *mut c_uchar,
) -> c_int {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    fn urandom_rng(buf: &mut [u8], len: u64) {
        randombytes::randombytes_urandom(buf, len);
    }
    sign::crypto_sign_keypair_with_rng(pk_s, sk_s, urandom_rng)
}

// ---- Signing ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    fn urandom_rng(buf: &mut [u8], len: u64) {
        randombytes::randombytes_urandom(buf, len);
    }
    let ret = sign::crypto_sign_signature_impl(sig_s, unsafe { &mut *siglen }, m_s, mlen, sk_s, urandom_rng);
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    let sig_s = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify_impl(sig_s, siglen, m_s, mlen, pk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut c_uchar, smlen: *mut u64,
    m: *const c_uchar, mlen: u64, sk: *const c_uchar,
) -> c_int {
    let sm_s = unsafe { std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    fn urandom_rng(buf: &mut [u8], len: u64) {
        randombytes::randombytes_urandom(buf, len);
    }
    let mut siglen: usize = 0;
    sign::crypto_sign_signature_impl(sm_s, &mut siglen, m_s, mlen as usize, sk_s, urandom_rng);
    // memmove sm + SPX_BYTES <- m
    unsafe {
        std::ptr::copy(m, sm.add(params::SPX_BYTES), mlen as usize);
        *smlen = siglen as u64 + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut c_uchar, mlen: *mut u64,
    sm: *const c_uchar, smlen: u64, pk: *const c_uchar,
) -> c_int {
    if smlen < params::SPX_BYTES as u64 {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }
    let real_mlen = smlen - params::SPX_BYTES as u64;
    let sm_s = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };

    if sign::crypto_sign_verify_impl(
        &sm_s[..params::SPX_BYTES], params::SPX_BYTES,
        &sm_s[params::SPX_BYTES..], real_mlen as usize, pk_s,
    ) != 0 {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen as usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe {
        *mlen = real_mlen;
        std::ptr::copy(sm.add(params::SPX_BYTES), m, real_mlen as usize);
    }
    0
}

// ---- RNG exports (deterministic DRBG) ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar,
    personalization_string: *mut c_uchar,
) {
    let ei = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) as &[u8] })
    };
    rng::randombytes_init(ei, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut c_uchar, xlen: u64) -> c_int {
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(buf, xlen)
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
    let ctx_ref = unsafe { &mut *ctx };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, 32) };
    let div_s = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx_ref, seed_s, div_s, maxlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut c_uchar,
    xlen: u64,
) -> c_int {
    let ctx_ref = unsafe { &mut *ctx };
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx_ref, buf, xlen)
}

// ---- initialize_hash_function export ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_hash_function(ctx: *mut context::SpxCtx) {
    hash::initialize_hash_function(unsafe { &mut *ctx });
}
