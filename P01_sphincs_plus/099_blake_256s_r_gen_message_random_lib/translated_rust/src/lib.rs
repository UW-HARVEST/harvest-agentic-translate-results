#![allow(non_snake_case, unused_unsafe, clippy::missing_safety_doc)]

mod params;
mod context;
mod address;
mod utils;
mod blake256;
mod blake512;
mod hash_blake;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod randombytes;
mod sign;

use params::*;
use std::slice;

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
    let pk = unsafe { slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk = unsafe { slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig = unsafe { slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let siglen = unsafe { &mut *siglen };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let sk = unsafe { slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let sig = unsafe { slice::from_raw_parts(sig, siglen) };
    let m = unsafe { slice::from_raw_parts(m, mlen) };
    let pk = unsafe { slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let sm = unsafe { slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };
    let smlen = unsafe { &mut *smlen };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };

    let mut siglen: usize = 0;
    sign::crypto_sign_signature(sm, &mut siglen, m, mlen as usize, sk);

    // memmove sm + SPX_BYTES <- m
    sm.copy_within(0..0, 0); // no-op, we need to copy m into sm+SPX_BYTES
    // Actually we need to copy m into the buffer after the signature
    sm[SPX_BYTES..SPX_BYTES + mlen as usize].copy_from_slice(m);
    *smlen = (siglen as u64) + mlen;
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let sm = unsafe { slice::from_raw_parts(sm, smlen as usize) };
    let m = unsafe { slice::from_raw_parts_mut(m, smlen as usize) };
    let mlen = unsafe { &mut *mlen };
    let pk = unsafe { slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };

    if (smlen as usize) < SPX_BYTES {
        for b in m.iter_mut() { *b = 0; }
        *mlen = 0;
        return -1;
    }

    *mlen = smlen - SPX_BYTES as u64;

    if sign::crypto_sign_verify(sm, SPX_BYTES, &sm[SPX_BYTES..], *mlen as usize, pk) != 0 {
        for b in m.iter_mut() { *b = 0; }
        *mlen = 0;
        return -1;
    }

    // memmove m <- sm + SPX_BYTES
    let msg_len = *mlen as usize;
    m[..msg_len].copy_from_slice(&sm[SPX_BYTES..SPX_BYTES + msg_len]);
    0
}
