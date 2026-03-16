#![allow(clippy::missing_safety_doc)]

mod address;
mod blake256;
mod blake512;
mod context;
mod fors;
mod hash_blake;
mod merkle;
mod params;
mod rng;
mod sign;
mod thash;
mod utils;
mod utilsx1;
mod wots;
mod wotsx1;

use params::*;

// --- Public API: size query functions ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> libc::c_ulonglong {
    CRYPTO_SECRETKEYBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> libc::c_ulonglong {
    CRYPTO_PUBLICKEYBYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> libc::c_ulonglong {
    CRYPTO_BYTES as libc::c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> libc::c_ulonglong {
    CRYPTO_SEEDBYTES as libc::c_ulonglong
}

// --- Key generation ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut libc::c_uchar,
    sk: *mut libc::c_uchar,
    seed: *const libc::c_uchar,
) -> libc::c_int {
    let pk_s = unsafe { core::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk_s = unsafe { core::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed_s = unsafe { core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(
    pk: *mut libc::c_uchar,
    sk: *mut libc::c_uchar,
) -> libc::c_int {
    let pk_s = unsafe { core::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk_s = unsafe { core::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    sign::crypto_sign_keypair_impl(pk_s, sk_s)
}

// --- Signing ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut libc::size_t,
    m: *const u8,
    mlen: libc::size_t,
    sk: *const u8,
) -> libc::c_int {
    let sig_s = unsafe { core::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m_s = unsafe { core::slice::from_raw_parts(m, mlen) };
    let sk_s = unsafe { core::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let mut sl: usize = 0;
    let ret = sign::crypto_sign_signature_impl(sig_s, &mut sl, m_s, mlen, sk_s);
    unsafe { *siglen = sl; }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: libc::size_t,
    m: *const u8,
    mlen: libc::size_t,
    pk: *const u8,
) -> libc::c_int {
    let sig_s = unsafe { core::slice::from_raw_parts(sig, siglen) };
    let m_s = unsafe { core::slice::from_raw_parts(m, mlen) };
    let pk_s = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    sign::crypto_sign_verify_impl(sig_s, siglen, m_s, mlen, pk_s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut libc::c_uchar,
    smlen: *mut libc::c_ulonglong,
    m: *const libc::c_uchar,
    mlen: libc::c_ulonglong,
    sk: *const libc::c_uchar,
) -> libc::c_int {
    let sm_s = unsafe { core::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let m_s = unsafe { core::slice::from_raw_parts(m, mlen as usize) };
    let sk_s = unsafe { core::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let mut sl: u64 = 0;
    let ret = sign::crypto_sign_impl(sm_s, &mut sl, m_s, mlen, sk_s);
    unsafe { *smlen = sl; }
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut libc::c_uchar,
    mlen: *mut libc::c_ulonglong,
    sm: *const libc::c_uchar,
    smlen: libc::c_ulonglong,
    pk: *const libc::c_uchar,
) -> libc::c_int {
    let m_s = unsafe { core::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm_s = unsafe { core::slice::from_raw_parts(sm, smlen as usize) };
    let pk_s = unsafe { core::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let mut ml: u64 = 0;
    let ret = sign::crypto_sign_open_impl(m_s, &mut ml, sm_s, smlen, pk_s);
    unsafe { *mlen = ml; }
    ret
}

// --- RNG functions ---

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut libc::c_uchar,
    personalization_string: *mut libc::c_uchar,
) {
    let ei = unsafe { core::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(ei, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(
    x: *mut libc::c_uchar,
    xlen: libc::c_ulonglong,
) -> libc::c_int {
    let x_s = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes_rng(x_s, xlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut libc::c_uchar,
    diversifier: *mut libc::c_uchar,
    maxlen: libc::c_ulong,
) -> libc::c_int {
    let ctx_r = unsafe { &mut *ctx };
    let seed_s = unsafe { core::slice::from_raw_parts(seed, 32) };
    let div_s = unsafe { core::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx_r, seed_s, div_s, maxlen as u64)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut libc::c_uchar,
    xlen: libc::c_ulong,
) -> libc::c_int {
    let ctx_r = unsafe { &mut *ctx };
    let x_s = unsafe { core::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx_r, x_s, xlen as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut libc::c_uchar,
    key: *mut libc::c_uchar,
    v: *mut libc::c_uchar,
) {
    let key_arr = unsafe { &mut *(key as *mut [u8; 32]) };
    let v_arr = unsafe { &mut *(v as *mut [u8; 16]) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(provided_data, 48) })
    };
    rng::aes256_ctr_drbg_update(pd, key_arr, v_arr);
}
