#![allow(clippy::all)]
#![allow(unused_imports, unused_variables, dead_code, non_snake_case)]

mod params;
mod context;
mod sha2;
mod utils;
mod address;
mod hash;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod sign;
mod randombytes;

use params::*;
use std::ffi::c_int;

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
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let siglen = unsafe { &mut *siglen };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> c_int {
    let sm = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };

    let mut siglen: usize = 0;
    sign::crypto_sign_signature(sm, &mut siglen, m_slice, mlen as usize, sk);

    // memmove sm + SPX_BYTES <- m
    sm.copy_within(..0, 0); // no-op, we need to copy m into sm+SPX_BYTES
    // Actually we need to copy from the original m pointer
    sm[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m_slice);
    unsafe { *smlen = (siglen as u64) + mlen };
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> c_int {
    let smlen_usize = smlen as usize;
    if smlen_usize < SPX_BYTES {
        let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
        for b in m_slice.iter_mut() { *b = 0; }
        unsafe { *mlen = 0 };
        return -1;
    }

    let sm_slice = unsafe { std::slice::from_raw_parts(sm, smlen_usize) };
    let pk_slice = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };

    let msg_len = smlen_usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64 };

    if sign::crypto_sign_verify(sm_slice, SPX_BYTES, &sm_slice[SPX_BYTES..], msg_len, pk_slice) != 0 {
        let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
        for b in m_slice.iter_mut() { *b = 0; }
        unsafe { *mlen = 0 };
        return -1;
    }

    // memmove m <- sm + SPX_BYTES
    let m_slice = unsafe { std::slice::from_raw_parts_mut(m, msg_len) };
    m_slice.copy_from_slice(&sm_slice[SPX_BYTES..SPX_BYTES + msg_len]);
    0
}
