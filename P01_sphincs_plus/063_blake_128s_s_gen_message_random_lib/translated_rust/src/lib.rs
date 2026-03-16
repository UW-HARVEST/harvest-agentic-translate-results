#![allow(clippy::too_many_arguments)]
#![allow(unused_unsafe)]

pub mod params;
pub mod context;
pub mod address;
pub mod utils;
pub mod blake256;
pub mod blake512;
pub mod hash_blake;
pub mod thash_blake_simple;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod utilsx1;
pub mod merkle;
pub mod rng;
pub mod sign;

use std::ffi::c_int;
use std::os::raw::c_uchar;

// ---- Public C API ----

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
    pk: *mut c_uchar,
    sk: *mut c_uchar,
    seed: *const c_uchar,
) -> c_int {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES);
        let seed_s = std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES);
        sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
) -> c_int {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, params::SPX_PK_BYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, params::SPX_SK_BYTES);
        sign::crypto_sign_keypair(pk_s, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, params::SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sk_s = std::slice::from_raw_parts(sk, params::SPX_SK_BYTES);
        sign::crypto_sign_signature(sig_s, &mut *siglen, m_s, mlen, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = std::slice::from_raw_parts(sig, siglen);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, params::SPX_PK_BYTES);
        sign::crypto_sign_verify(sig_s, siglen, m_s, mlen, pk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut c_uchar,
    smlen: *mut u64,
    m: *const c_uchar,
    mlen: u64,
    sk: *const c_uchar,
) -> c_int {
    unsafe {
        let sm_s = std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        let sk_s = std::slice::from_raw_parts(sk, params::SPX_SK_BYTES);
        sign::crypto_sign(sm_s, &mut *smlen, m_s, mlen, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut u64,
    sm: *const c_uchar,
    smlen: u64,
    pk: *const c_uchar,
) -> c_int {
    unsafe {
        let m_s = std::slice::from_raw_parts_mut(m, smlen as usize);
        let sm_s = std::slice::from_raw_parts(sm, smlen as usize);
        let pk_s = std::slice::from_raw_parts(pk, params::SPX_PK_BYTES);
        sign::crypto_sign_open(m_s, &mut *mlen, sm_s, smlen, pk_s)
    }
}

// RNG C API
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar,
    personalization_string: *mut c_uchar,
) {
    unsafe {
        let ei = std::slice::from_raw_parts(entropy_input, 48);
        let ps = if personalization_string.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(personalization_string, 48) as &[u8])
        };
        rng::randombytes_init(ei, ps);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut c_uchar, xlen: u64) -> c_int {
    unsafe {
        let x_s = std::slice::from_raw_parts_mut(x, xlen as usize);
        rng::randombytes(x_s, xlen)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar,
    key: *mut c_uchar,
    v: *mut c_uchar,
) {
    unsafe {
        let key_s = std::slice::from_raw_parts_mut(key, 32);
        let v_s = std::slice::from_raw_parts_mut(v, 16);
        let pd = if provided_data.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(provided_data, 48) as &[u8])
        };
        rng::aes256_ctr_drbg_update(pd, key_s, v_s);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut c_uchar,
    diversifier: *mut c_uchar,
    maxlen: u64,
) -> c_int {
    unsafe {
        let seed_s = std::slice::from_raw_parts(seed, 32);
        let div_s = std::slice::from_raw_parts(diversifier, 8);
        rng::seedexpander_init(&mut *ctx, seed_s, div_s, maxlen)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut c_uchar,
    xlen: u64,
) -> c_int {
    unsafe {
        let x_s = std::slice::from_raw_parts_mut(x, xlen as usize);
        rng::seedexpander(&mut *ctx, x_s, xlen as usize)
    }
}
