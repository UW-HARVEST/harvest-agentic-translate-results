#![allow(clippy::missing_safety_doc, dead_code, unused_imports)]

mod params;
mod context;
mod address;
mod sha2;
mod hash;
mod thash;
mod utils;
mod wots;
mod wotsx1;
mod utilsx1;
mod fors;
mod merkle;
mod rng;
mod sign;

use std::os::raw::{c_uchar, c_int, c_ulonglong, c_ulong};
use std::slice;

// ---- sign.c exports ----

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
    pk: *mut c_uchar, sk: *mut c_uchar, seed: *const c_uchar,
) -> c_int {
    let pk = unsafe { slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(
    pk: *mut c_uchar, sk: *mut c_uchar,
) -> c_int {
    let pk = unsafe { slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk, sk) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    let sig = unsafe { slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let sk = unsafe { slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let siglen = unsafe { &mut *siglen };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    let sig = unsafe { slice::from_raw_parts(sig, siglen) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let pk = unsafe { slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut c_uchar, smlen: *mut c_ulonglong,
    m: *const c_uchar, mlen: c_ulonglong, sk: *const c_uchar,
) -> c_int {
    let mlen_usize = mlen as usize;
    let sm = unsafe { slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen_usize) };
    let m = unsafe { slice::from_raw_parts(m, mlen_usize) };
    let sk = unsafe { slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let smlen = unsafe { &mut *smlen };
    sign::crypto_sign_impl(sm, smlen, m, mlen, sk) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut c_uchar, mlen: *mut c_ulonglong,
    sm: *const c_uchar, smlen: c_ulonglong, pk: *const c_uchar,
) -> c_int {
    let smlen_usize = smlen as usize;
    let m = unsafe { slice::from_raw_parts_mut(m, smlen_usize) };
    let sm = unsafe { slice::from_raw_parts(sm, smlen_usize) };
    let pk = unsafe { slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mlen = unsafe { &mut *mlen };
    sign::crypto_sign_open_impl(m, mlen, sm, smlen, pk) as c_int
}

// ---- rng.c exports ----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar, key: *mut c_uchar, v: *mut c_uchar,
) {
    let key = unsafe { slice::from_raw_parts_mut(key, 32) };
    let v = unsafe { slice::from_raw_parts_mut(v, 16) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(provided_data, 48) } as &[u8])
    };
    rng::aes256_ctr_drbg_update(pd, key, v);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct, seed: *mut c_uchar,
    diversifier: *mut c_uchar, maxlen: c_ulong,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx, seed, diversifier, maxlen as u64) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct, x: *mut c_uchar, xlen: c_ulong,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let xlen_usize = xlen as usize;
    let x = unsafe { slice::from_raw_parts_mut(x, xlen_usize) };
    rng::seedexpander(ctx, x, xlen_usize) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar, personalization_string: *mut c_uchar,
) {
    let entropy = unsafe { slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(personalization_string, 48) } as &[u8])
    };
    rng::randombytes_init_global(entropy, ps);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut c_uchar, xlen: c_ulonglong) -> c_int {
    let xlen_usize = xlen as usize;
    let x = unsafe { slice::from_raw_parts_mut(x, xlen_usize) };
    rng::randombytes_global(x, xlen_usize) as c_int
}
