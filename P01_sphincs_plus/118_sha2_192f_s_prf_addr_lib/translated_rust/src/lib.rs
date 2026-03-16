mod params;
mod context;
mod sha2;
mod utils;
mod hash;
mod thash;
mod wots;
mod wotsx1;
mod fors;
mod utilsx1;
mod merkle;
mod rng;
mod sign;

use params::*;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    SPX_SK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    SPX_PK_BYTES as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    SPX_BYTES as u64
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
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        let seed_s = std::slice::from_raw_parts(seed, CRYPTO_SEEDBYTES);
        sign::crypto_sign_seed_keypair(pk_s, sk_s, seed_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    unsafe {
        let pk_s = std::slice::from_raw_parts_mut(pk, SPX_PK_BYTES);
        let sk_s = std::slice::from_raw_parts_mut(sk, SPX_SK_BYTES);
        sign::crypto_sign_keypair_internal(pk_s, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_signature(
    sig: *mut u8,
    siglen: *mut usize,
    m: *const u8,
    mlen: usize,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sig_s = std::slice::from_raw_parts_mut(sig, SPX_BYTES);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);
        sign::crypto_sign_signature_internal(sig_s, &mut *siglen, m_s, mlen, sk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_verify(
    sig: *const u8,
    siglen: usize,
    m: *const u8,
    mlen: usize,
    pk: *const u8,
) -> i32 {
    unsafe {
        let sig_s = std::slice::from_raw_parts(sig, siglen);
        let m_s = std::slice::from_raw_parts(m, mlen);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);
        sign::crypto_sign_verify_internal(sig_s, siglen, m_s, mlen, pk_s)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut u8,
    smlen: *mut u64,
    m: *const u8,
    mlen: u64,
    sk: *const u8,
) -> i32 {
    unsafe {
        let sm_s = std::slice::from_raw_parts_mut(sm, SPX_BYTES + mlen as usize);
        let m_s = std::slice::from_raw_parts(m, mlen as usize);
        let sk_s = std::slice::from_raw_parts(sk, SPX_SK_BYTES);

        let mut siglen: usize = 0;
        sign::crypto_sign_signature_internal(sm_s, &mut siglen, m_s, mlen as usize, sk_s);

        // memmove sm + SPX_BYTES <- m
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen as usize);
        *smlen = (siglen as u64) + mlen;
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut u8,
    mlen: *mut u64,
    sm: *const u8,
    smlen: u64,
    pk: *const u8,
) -> i32 {
    unsafe {
        let smlen_usize = smlen as usize;
        if smlen_usize < SPX_BYTES {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        let actual_mlen = smlen_usize - SPX_BYTES;
        *mlen = actual_mlen as u64;

        let sm_s = std::slice::from_raw_parts(sm, smlen_usize);
        let pk_s = std::slice::from_raw_parts(pk, SPX_PK_BYTES);

        if sign::crypto_sign_verify_internal(
            &sm_s[..SPX_BYTES],
            SPX_BYTES,
            &sm_s[SPX_BYTES..],
            actual_mlen,
            pk_s,
        ) != 0
        {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
            return -1;
        }

        std::ptr::copy(sm.add(SPX_BYTES), m, actual_mlen);
        0
    }
}
