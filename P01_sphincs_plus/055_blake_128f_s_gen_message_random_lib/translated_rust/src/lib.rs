mod params;
mod context;
mod address;
mod blake256;
mod hash;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod randombytes;
mod sign;

use std::ffi::c_int;

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
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut u8, sk: *mut u8, seed: *const u8,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_keypair_internal(pk, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8, siglen: *mut usize,
    m: *const u8, mlen: usize, sk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, SPX_BYTES) };
    let siglen = unsafe { &mut *siglen };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    sign::crypto_sign_signature_internal(sig, siglen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8, siglen: usize,
    m: *const u8, mlen: usize, pk: *const u8,
) -> c_int {
    let sig = unsafe { std::slice::from_raw_parts(sig, siglen) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify_internal(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8, smlen: *mut u64,
    m: *const u8, mlen: u64, sk: *const u8,
) -> c_int {
    let m_slice = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk_slice = unsafe { std::slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };
    let sm_slice = unsafe { std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize) };

    let mut siglen: usize = 0;
    sign::crypto_sign_signature_internal(sm_slice, &mut siglen, m_slice, mlen as usize, sk_slice);

    // memmove(sm + SPX_BYTES, m, mlen)
    // sm_slice already points to sm, copy m after signature
    unsafe {
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8, mlen: *mut u64,
    sm: *const u8, smlen: u64, pk: *const u8,
) -> c_int {
    let smlen_usize = smlen as usize;

    if smlen_usize < SPX_BYTES {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    let real_mlen = smlen_usize - SPX_BYTES;
    unsafe { *mlen = real_mlen as u64; }

    let sig = unsafe { std::slice::from_raw_parts(sm, SPX_BYTES) };
    let msg = unsafe { std::slice::from_raw_parts(sm.add(SPX_BYTES), real_mlen) };
    let pk = unsafe { std::slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };

    if sign::crypto_sign_verify_internal(sig, SPX_BYTES, msg, real_mlen, pk) != 0 {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    unsafe {
        std::ptr::copy(sm.add(SPX_BYTES), m, real_mlen);
    }
    0
}
