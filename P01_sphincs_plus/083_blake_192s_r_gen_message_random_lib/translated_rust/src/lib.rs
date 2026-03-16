#![allow(clippy::all)]
#![allow(unused_unsafe)]

mod params;
mod context;
mod address;
mod blake256;
mod blake512;
mod hash_blake;
mod thash;
mod wots;
mod fors;
mod merkle;
mod sign;
mod rng;
mod randombytes;

use std::ptr;
use params::*;

// ---- sign.c API ----

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
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> i32 {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk_s = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk_s = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    sign::crypto_sign_keypair_internal(pk_s, sk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> i32 {
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let mut sl: usize = 0;
    let ret = sign::crypto_sign_signature_internal(sig_s, &mut sl, m_s, mlen, sk_s);
    unsafe { *siglen = sl; }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> i32 {
    let sig_s = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    sign::crypto_sign_verify_internal(sig_s, siglen, m_s, mlen, pk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> i32 {
    let sig_s = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let m_s = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let mut siglen: usize = 0;
    sign::crypto_sign_signature_internal(sig_s, &mut siglen, m_s, mlen as usize, sk_s);
    // memmove sm + SPX_BYTES <- m
    unsafe {
        ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> i32 {
    let smlen_usize = smlen as usize;
    if smlen_usize < SPX_BYTES {
        unsafe {
            ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    let msg_len = smlen_usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64; }

    let sig_s = unsafe { std::slice::from_raw_parts(sm, SPX_BYTES) };
    let msg_s = unsafe { std::slice::from_raw_parts(sm.add(SPX_BYTES), msg_len) };
    let pk_s = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };

    if sign::crypto_sign_verify_internal(sig_s, SPX_BYTES, msg_s, msg_len, pk_s) != 0 {
        unsafe {
            ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe { ptr::copy(sm.add(SPX_BYTES), m, msg_len); }
    0
}

// ---- rng.c API ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut u8, personalization_string: *mut u8,
) {
    let ei = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) as &[u8] })
    };
    rng::randombytes_init_rng(ei, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes_rng(x_s, xlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut u8, key: *mut u8, v: *mut u8,
) {
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) as &[u8] })
    };
    let key_s = unsafe { std::slice::from_raw_parts_mut(key, 32) };
    let v_s = unsafe { std::slice::from_raw_parts_mut(v, 16) };
    rng::aes256_ctr_drbg_update(pd, key_s, v_s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct, seed: *mut u8, diversifier: *mut u8, maxlen: u64,
) -> i32 {
    let ctx_r = unsafe { &mut *ctx };
    let seed_s = unsafe { std::slice::from_raw_parts(seed, 32) };
    let div_s = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx_r, seed_s, div_s, maxlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct, x: *mut u8, xlen: u64,
) -> i32 {
    let ctx_r = unsafe { &mut *ctx };
    let x_s = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx_r, x_s, xlen)
}
