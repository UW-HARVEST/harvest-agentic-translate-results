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
mod wots;

use std::ffi::c_int;
use std::os::raw::c_uchar;

// --- Public C API ---

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_secretkeybytes() -> u64 {
    sign::crypto_sign_secretkeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_publickeybytes() -> u64 {
    sign::crypto_sign_publickeybytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_bytes() -> u64 {
    sign::crypto_sign_bytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seedbytes() -> u64 {
    sign::crypto_sign_seedbytes()
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_seed_keypair(
    pk: *mut c_uchar,
    sk: *mut c_uchar,
    seed: *const c_uchar,
) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
    let seed = unsafe { std::slice::from_raw_parts(seed, params::CRYPTO_SEEDBYTES) };
    sign::crypto_sign_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_keypair(pk: *mut c_uchar, sk: *mut c_uchar) -> c_int {
    let pk = unsafe { std::slice::from_raw_parts_mut(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let sk = unsafe { std::slice::from_raw_parts_mut(sk, params::CRYPTO_SECRETKEYBYTES) };
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
    let sig = unsafe { std::slice::from_raw_parts_mut(sig, params::SPX_BYTES) };
    let m = unsafe { std::slice::from_raw_parts(m, mlen) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
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
    let pk = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    sign::crypto_sign_verify(sig, siglen, m, mlen, pk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign(
    sm: *mut c_uchar,
    smlen: *mut u64,
    m: *const c_uchar,
    mlen: u64,
    sk: *const c_uchar,
) -> c_int {
    let sm = unsafe {
        std::slice::from_raw_parts_mut(sm, params::SPX_BYTES + mlen as usize)
    };
    let m = unsafe { std::slice::from_raw_parts(m, mlen as usize) };
    let sk = unsafe { std::slice::from_raw_parts(sk, params::CRYPTO_SECRETKEYBYTES) };
    let smlen = unsafe { &mut *smlen };
    sign::crypto_sign(sm, smlen, m, mlen, sk)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_sign_open(
    m: *mut c_uchar,
    mlen: *mut u64,
    sm: *const c_uchar,
    smlen: u64,
    pk: *const c_uchar,
) -> c_int {
    let m_out = unsafe { std::slice::from_raw_parts_mut(m, smlen as usize) };
    let sm = unsafe { std::slice::from_raw_parts(sm, smlen as usize) };
    let pk = unsafe { std::slice::from_raw_parts(pk, params::CRYPTO_PUBLICKEYBYTES) };
    let mlen = unsafe { &mut *mlen };
    sign::crypto_sign_open(m_out, mlen, sm, smlen, pk)
}

// --- RNG API ---

#[unsafe(no_mangle)]
pub extern "C" fn randombytes_init(
    entropy_input: *mut c_uchar,
    personalization_string: *mut c_uchar,
) {
    let entropy = unsafe { std::slice::from_raw_parts(entropy_input, 48) };
    let ps = if personalization_string.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(personalization_string, 48) })
    };
    rng::randombytes_init(entropy, ps);
}

#[unsafe(no_mangle)]
pub extern "C" fn randombytes(x: *mut c_uchar, xlen: u64) -> c_int {
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::randombytes(buf, xlen);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn AES256_CTR_DRBG_Update(
    provided_data: *mut c_uchar,
    key: *mut c_uchar,
    v: *mut c_uchar,
) {
    let key_slice = unsafe { &mut *(key as *mut [u8; 32]) };
    let v_slice = unsafe { &mut *(v as *mut [u8; 16]) };
    let data = if provided_data.is_null() {
        None
    } else {
        Some(unsafe { std::slice::from_raw_parts(provided_data, 48) })
    };
    rng::aes256_ctr_drbg_update(data, key_slice, v_slice);
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander_init(
    ctx: *mut rng::AesXofStruct,
    seed: *mut c_uchar,
    diversifier: *mut c_uchar,
    maxlen: u64,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let seed = unsafe { std::slice::from_raw_parts(seed, 32) };
    let diversifier = unsafe { std::slice::from_raw_parts(diversifier, 8) };
    rng::seedexpander_init(ctx, seed, diversifier, maxlen)
}

#[unsafe(no_mangle)]
pub extern "C" fn seedexpander(
    ctx: *mut rng::AesXofStruct,
    x: *mut c_uchar,
    xlen: u64,
) -> c_int {
    let ctx = unsafe { &mut *ctx };
    let buf = unsafe { std::slice::from_raw_parts_mut(x, xlen as usize) };
    rng::seedexpander(ctx, buf, xlen as usize)
}
