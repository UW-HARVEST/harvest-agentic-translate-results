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
mod utilsx1;
mod wots;
mod wotsx1;

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
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, SPX_SK_BYTES) };
    let smlen = unsafe { &mut *smlen };

    let mut siglen: usize = 0;
    sign::crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    // memmove(sm + SPX_BYTES, m, mlen)
    sm.copy_within(..0, 0); // no-op, just for clarity
    unsafe {
        std::ptr::copy(m.as_ptr(), sm.as_mut_ptr().add(SPX_BYTES), mlen as usize);
    }
    *smlen = siglen as u64 + mlen;

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
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen_usize) };
    let m_slice = unsafe { std::slice::from_raw_parts_mut(m, smlen_usize) };
    let pk = unsafe { std::slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let mlen = unsafe { &mut *mlen };

    if smlen_usize < SPX_BYTES {
        for i in 0..smlen_usize {
            m_slice[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    *mlen = (smlen_usize - SPX_BYTES) as u64;

    if sign::crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for i in 0..smlen_usize {
            m_slice[i] = 0;
        }
        *mlen = 0;
        return -1;
    }

    // memmove(m, sm + SPX_BYTES, *mlen)
    unsafe {
        std::ptr::copy(
            sm.as_ptr().add(SPX_BYTES),
            m_slice.as_mut_ptr(),
            *mlen as usize,
        );
    }

    0
}
