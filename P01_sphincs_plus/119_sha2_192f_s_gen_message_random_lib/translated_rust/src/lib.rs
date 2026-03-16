#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]

mod address;
mod context;
mod fors;
mod hash_sha2;
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
    let mlen_usize = mlen as usize;
    let sm_slice = unsafe { slice::from_raw_parts_mut(sm, SPX_BYTES + mlen_usize) };
    let m_slice = unsafe { slice::from_raw_parts(m, mlen_usize) };
    let sk_slice = unsafe { slice::from_raw_parts(sk, CRYPTO_SECRETKEYBYTES) };

    let mut siglen: usize = 0;
    sign::crypto_sign_signature(sm_slice, &mut siglen, m_slice, mlen_usize, sk_slice);

    // memmove(sm + SPX_BYTES, m, mlen)
    unsafe {
        std::ptr::copy(m, sm.add(SPX_BYTES), mlen_usize);
    }
    unsafe { *smlen = (siglen + mlen_usize) as u64; }

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
    let smlen_usize = smlen as usize;

    if smlen_usize < SPX_BYTES {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    let msg_len = smlen_usize - SPX_BYTES;
    unsafe { *mlen = msg_len as u64; }

    let sm_slice = unsafe { slice::from_raw_parts(sm, smlen_usize) };
    let pk_slice = unsafe { slice::from_raw_parts(pk, CRYPTO_PUBLICKEYBYTES) };

    if sign::crypto_sign_verify(&sm_slice[..SPX_BYTES], SPX_BYTES, &sm_slice[SPX_BYTES..], msg_len, pk_slice) != 0 {
        unsafe {
            std::ptr::write_bytes(m, 0, smlen_usize);
            *mlen = 0;
        }
        return -1;
    }

    // memmove(m, sm + SPX_BYTES, *mlen)
    unsafe {
        std::ptr::copy(sm.add(SPX_BYTES), m, msg_len);
    }

    0
}

// Export gen_message_random
#[unsafe(no_mangle)]
pub extern "C" fn gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const context::SpxCtx,
) {
    let r = unsafe { slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf = unsafe { slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let ctx = unsafe { &*ctx };
    hash_sha2::gen_message_random(r, sk_prf, optrand, m, mlen, ctx);
}

// Export remaining hash functions
#[unsafe(no_mangle)]
pub extern "C" fn prf_addr(
    out: *mut u8,
    ctx: *const context::SpxCtx,
    addr: *const u32,
) {
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { &*(addr as *const [u32; 8]) };
    hash_sha2::prf_addr(out, ctx, addr);
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    ctx: *const context::SpxCtx,
) {
    let digest = unsafe { slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let tree = unsafe { &mut *tree };
    let leaf_idx = unsafe { &mut *leaf_idx };
    let r = unsafe { slice::from_raw_parts(r, SPX_N) };
    let pk = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };
    let ctx = unsafe { &*ctx };
    hash_sha2::hash_message(digest, tree, leaf_idx, r, pk, m, mlen, ctx);
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_hash_function(ctx: *mut context::SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    hash_sha2::initialize_hash_function(ctx);
}

// Export randombytes functions
#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *const u8,
    personalization_string: *const u8,
) {
    let entropy = unsafe { slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(entropy, ps);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut u8, xlen: u64) -> i32 {
    let x = unsafe { slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(x, xlen);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *const u8,
    key: *mut u8,
    v: *mut u8,
) {
    let key_slice = unsafe { slice::from_raw_parts_mut(key, 32) };
    let v_slice = unsafe { slice::from_raw_parts_mut(v, 16) };
    let pd = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(provided_data, 48) })
    };
    rng::aes256_ctr_drbg_update_export(pd, key_slice, v_slice);
}
