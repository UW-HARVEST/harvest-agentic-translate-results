// SPHINCS+ Rust translation entry point.
//
// Each (HASH_BACKEND, THASH, SECPAR) combination corresponds to one possible
// CMake build configuration in the original C source.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_range_loop)]

pub mod address;
pub mod context;
pub mod params;
pub mod rng;
pub mod sign;
pub mod thash;
pub mod utils;
pub mod fors;
pub mod hash;
pub mod merkle;
pub mod utilsx1;
pub mod wots;
pub mod wotsx1;

#[cfg(feature = "haraka")]
pub mod haraka;
#[cfg(feature = "haraka")]
pub mod hash_haraka;
#[cfg(feature = "haraka")]
pub mod thash_haraka;

#[cfg(feature = "sha2")]
pub mod sha2;
#[cfg(feature = "sha2")]
pub mod hash_sha2;
#[cfg(feature = "sha2")]
pub mod thash_sha2;

#[cfg(feature = "shake")]
pub mod fips202;
#[cfg(feature = "shake")]
pub mod hash_shake;
#[cfg(feature = "shake")]
pub mod thash_shake;

#[cfg(feature = "blake")]
pub mod blake;
#[cfg(feature = "blake")]
pub mod hash_blake;
#[cfg(feature = "blake")]
pub mod thash_blake;

// ---------------------------------------------------------------------------
// C ABI: extern "C" entry points exported by the cdylib.
//
// The original C headers do not wrap the api.h symbols with SPX_NAMESPACE, so
// the linker symbols are simply `crypto_sign_keypair`, `crypto_sign`, etc.
// ---------------------------------------------------------------------------

use std::os::raw::{c_int, c_uchar, c_ulonglong};

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> c_ulonglong {
    sign::crypto_sign_secretkeybytes() as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> c_ulonglong {
    sign::crypto_sign_publickeybytes() as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> c_ulonglong {
    sign::crypto_sign_bytes() as c_ulonglong
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> c_ulonglong {
    sign::crypto_sign_seedbytes() as c_ulonglong
}

/// # Safety
/// `pk` must point to at least `CRYPTO_PUBLICKEYBYTES` writable bytes,
/// `sk` must point to at least `CRYPTO_SECRETKEYBYTES` writable bytes,
/// `seed` must point to at least `CRYPTO_SEEDBYTES` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
    seed: *const c_uchar,
) -> c_int {
    let pk = std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES);
    let sk = std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES);
    let seed = std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES);
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    let pk = std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES);
    let sk = std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES);
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig = std::slice::from_raw_parts_mut(sig, params::CRYPTO_BYTES);
    let m = std::slice::from_raw_parts(m, mlen);
    let sk = std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES);
    let mut siglen_local = 0usize;
    let r = sign::crypto_sign_signature(sig, &mut siglen_local, m, mlen, sk);
    *siglen = siglen_local;
    r
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let sig = std::slice::from_raw_parts(sig, siglen);
    let m = std::slice::from_raw_parts(m, mlen);
    let pk = std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES);
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut c_uchar,
    smlen: *mut c_ulonglong,
    m: *const c_uchar,
    mlen: c_ulonglong,
    sk: *const c_uchar,
) -> c_int {
    let sm = std::slice::from_raw_parts_mut(sm, params::CRYPTO_BYTES + mlen as usize);
    let m = std::slice::from_raw_parts(m, mlen as usize);
    let sk = std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES);
    let mut smlen_local: u64 = 0;
    let r = sign::crypto_sign(sm, &mut smlen_local, m, mlen, sk);
    *smlen = smlen_local as c_ulonglong;
    r
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut c_ulonglong,
    sm: *const c_uchar,
    smlen: c_ulonglong,
    pk: *const c_uchar,
) -> c_int {
    let m_buf = std::slice::from_raw_parts_mut(m, smlen as usize);
    let sm = std::slice::from_raw_parts(sm, smlen as usize);
    let pk = std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES);
    let mut mlen_local: u64 = 0;
    let r = sign::crypto_sign_open(m_buf, &mut mlen_local, sm, smlen, pk);
    *mlen = mlen_local as c_ulonglong;
    r
}

// rng.h API (from NIST). These are not under SPX_NAMESPACE.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar,
    personalization_string: *mut c_uchar,
) {
    let ent = std::slice::from_raw_parts(entropy_input, 48);
    let mut ent_arr = [0u8; 48];
    ent_arr.copy_from_slice(ent);
    if personalization_string.is_null() {
        rng::randombytes_init(&ent_arr, None);
    } else {
        let p = std::slice::from_raw_parts(personalization_string, 48);
        let mut parr = [0u8; 48];
        parr.copy_from_slice(p);
        rng::randombytes_init(&ent_arr, Some(&parr));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn randombytes(x: *mut c_uchar, xlen: c_ulonglong) -> c_int {
    let x = std::slice::from_raw_parts_mut(x, xlen as usize);
    rng::randombytes(x, xlen) as c_int
}
