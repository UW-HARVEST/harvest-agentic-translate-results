#![allow(clippy::missing_safety_doc)]
#![allow(dead_code)]
#![allow(static_mut_refs)]

mod address;
mod compute_root_fn;
mod context;
mod fors;
mod hash;
mod merkle;
mod params;
mod randombytes;
mod sha2;
mod sign;
mod thash;
mod utils;
mod utilsx1;
mod wots;

use params::*;

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
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> i32 {
    let pk = unsafe { core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { core::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    let pk = unsafe { core::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { core::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    let sig = unsafe { core::slice::from_raw_parts_mut(sig, CRYPTO_BYTES) };
    let siglen = unsafe { &mut *siglen };
    let m = unsafe { core::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_signature(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    let sig = unsafe { core::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { core::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    let mlen_usize = mlen as usize;
    let sm_slice = unsafe { core::slice::from_raw_parts_mut(sm, CRYPTO_BYTES + mlen_usize) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen_usize) };
    let sk_slice = unsafe { core::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };

    let mut siglen: usize = 0;
    sign::crypto_sign_signature(sm_slice, &mut siglen, m_slice, mlen_usize, sk_slice);

    // memmove sm + SPX_BYTES, m, mlen
    unsafe {
        core::ptr::copy(m, sm.add(SPX_BYTES), mlen_usize);
        *smlen = (siglen as u64) + mlen;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    let smlen_usize = smlen as usize;

    if smlen_usize < SPX_BYTES {
        unsafe {
            core::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    let msg_len = smlen_usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64 };

    let sm_slice = unsafe { core::slice::from_raw_parts(sm, smlen_usize) };
    let pk_slice = unsafe { core::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };

    if sign::crypto_sign_verify(
        &sm_slice[..SPX_BYTES],
        SPX_BYTES,
        &sm_slice[SPX_BYTES..],
        msg_len,
        pk_slice,
    ) != 0
    {
        unsafe {
            core::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe {
        core::ptr::copy(sm.add(SPX_BYTES), m, msg_len);
    }

    0
}
