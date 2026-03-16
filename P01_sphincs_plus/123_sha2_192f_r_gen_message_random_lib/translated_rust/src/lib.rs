#![allow(
    non_upper_case_globals,
    unused_unsafe,
    unused_variables,
    unused_assignments,
    unused_mut,
    static_mut_refs,
    dead_code,
    clippy::missing_safety_doc,
)]

mod params;
mod context;
mod address;
mod sha2;
mod hash;
mod thash;
mod utils;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod rng;
mod sign;

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
    unsafe {
        let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        let seed_s = core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    unsafe {
        let pk_s = core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
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
) -> i32 {
    unsafe {
        let sig_s = core::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = core::slice::from_raw_parts(m, mlen);
        let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
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
) -> i32 {
    unsafe {
        let sig_s = core::slice::from_raw_parts(sig, if siglen > SPX_BYTES { siglen } else { SPX_BYTES });
        let m_s = core::slice::from_raw_parts(m, mlen);
        let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        sign::crypto_sign_verify(sig_s, siglen, m_s, mlen, pk_s)
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
        let sm_s = core::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize);
        let m_s = core::slice::from_raw_parts(m, mlen as usize);
        let sk_s = core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        sign::crypto_sign_impl(sm_s, &mut *smlen, m_s, mlen, sk_s)
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
        let m_s = core::slice::from_raw_parts_mut(m, smlen as usize);
        let sm_s = core::slice::from_raw_parts(sm, smlen as usize);
        let pk_s = core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        sign::crypto_sign_open_impl(m_s, &mut *mlen, sm_s, smlen, pk_s)
    }
}

// RNG API exports
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut u8,
    personalization_string: *mut u8,
) {
    unsafe {
        let ei = core::slice::from_raw_parts(entropy_input, 48);
        let ps = if personalization_string.is_null() {
            None
        } else {
            Some(core::slice::from_raw_parts(personalization_string, 48))
        };
        rng::randombytes_init(ei, ps);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    unsafe {
        let x_s = core::slice::from_raw_parts_mut(x, xlen as usize);
        rng::randombytes(x_s, xlen);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8,
    key: *mut u8,
    v: *mut u8,
) {
    unsafe {
        let k = &mut *(key as *mut [u8; 32]);
        let v_arr = &mut *(v as *mut [u8; 16]);
        let pd = if provided_data.is_null() {
            None
        } else {
            Some(core::slice::from_raw_parts(provided_data, 48) as &[u8])
        };
        rng::aes256_ctr_drbg_update(pd, k, v_arr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut u8,
    diversifier: *mut u8,
    maxlen: u64,
) -> i32 {
    unsafe {
        let seed_s = core::slice::from_raw_parts(seed, 32);
        let div_s = core::slice::from_raw_parts(diversifier, 8);
        rng::seedexpander_init(&mut *ctx, seed_s, div_s, maxlen)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut u8,
    xlen: u64,
) -> i32 {
    unsafe {
        let x_s = core::slice::from_raw_parts_mut(x, xlen as usize);
        rng::seedexpander(&mut *ctx, x_s, xlen)
    }
}
