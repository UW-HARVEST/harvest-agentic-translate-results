#![allow(non_snake_case, unused_unsafe, unused_imports, static_mut_refs, clippy::missing_safety_doc)]

pub mod params;
pub mod context;
pub mod blake256;
pub mod blake512;
pub mod hash_blake;
pub mod thash;
pub mod utils;
pub mod wots;
pub mod wotsx1;
pub mod fors;
pub mod merkle;
pub mod rng;
pub mod sign;

use std::ffi::c_int;
use std::os::raw::c_uchar;

use params::*;

// --- Public C API ---

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
    pk: *mut c_uchar, sk: *mut c_uchar, seed: *const c_uchar,
) -> c_int {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES);
        sign::crypto_sign_keypair_internal(pk_s, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sk_s = std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);
        sign::crypto_sign_signature_internal(sig_s, &mut *siglen, m_s, mlen, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    unsafe {
        let sig_s = std::slice::from_raw_parts(sig, if siglen > SPX_BYTES { siglen } else { SPX_BYTES });
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);
        sign::crypto_sign_verify_internal(sig_s, siglen, m_s, mlen, pk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut c_uchar, smlen: *mut u64,
    m: *const c_uchar, mlen: u64, sk: *const c_uchar,
) -> c_int {
    unsafe {
        let sm_s = std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        let sk_s = std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES);

        let mut siglen: usize = 0;
        sign::crypto_sign_signature_internal(sm_s, &mut siglen, m_s, mlen as usize, sk_s);

        // memmove(sm + SPX_BYTES, m, mlen)
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut c_uchar, mlen: *mut u64,
    sm: *const c_uchar, smlen: u64, pk: *const c_uchar,
) -> c_int {
    unsafe {
        let smlen_usize = smlen as usize;
        if smlen_usize < SPX_BYTES {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        let real_mlen = smlen_usize - SPX_BYTES;
        *mlen = real_mlen as u64;

        let sm_s = std::slice::from_raw_parts(sm, smlen_usize);
        let pk_s = std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES);

        if sign::crypto_sign_verify_internal(
            &sm_s[..SPX_BYTES], SPX_BYTES,
            &sm_s[SPX_BYTES..], real_mlen, pk_s,
        ) != 0 {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        std::ptr::copy(sm.add(SPX_BYTES), m, real_mlen);
        0
    }
}

// RNG exports
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

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar,
    key: *mut c_uchar,
    v: *mut c_uchar,
) {
    unsafe {
        let pd = if provided_data.is_null() {
            None
        } else {
            Some(std::slice::from_raw_parts(provided_data, 48) as &[u8])
        };
        let key_s = &mut *(key as *mut [u8; 32]);
        let v_s = &mut *(v as *mut [u8; 16]);
        rng::aes256_ctr_drbg_update(pd, key_s, v_s);
    }
}

// initialize_hash_function export
#[unsafe(no_mangle)]
pub extern "C" fn initialize_hash_function(ctx: *mut context::SpxCtx) {
    unsafe {
        hash_blake::initialize_hash_function(&mut *ctx);
    }
}
