pub mod r#ref;

// Translation of `crypto_kem/mlkem768/kem_mlkem768.c`.

use core::ffi::c_int;

use self::r#ref::{
    _sodium_mlkem768_ref_dec, _sodium_mlkem768_ref_enc, _sodium_mlkem768_ref_enc_deterministic,
    _sodium_mlkem768_ref_keypair, _sodium_mlkem768_ref_seed_keypair,
};

// include/sodium/crypto_kem_mlkem768.h
pub const crypto_kem_mlkem768_PUBLICKEYBYTES: usize = 1184;
pub const crypto_kem_mlkem768_SECRETKEYBYTES: usize = 2400;
pub const crypto_kem_mlkem768_CIPHERTEXTBYTES: usize = 1088;
pub const crypto_kem_mlkem768_SHAREDSECRETBYTES: usize = 32;
pub const crypto_kem_mlkem768_SEEDBYTES: usize = 64;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_publickeybytes() -> usize {
    crypto_kem_mlkem768_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_secretkeybytes() -> usize {
    crypto_kem_mlkem768_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_ciphertextbytes() -> usize {
    crypto_kem_mlkem768_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_sharedsecretbytes() -> usize {
    crypto_kem_mlkem768_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_seedbytes() -> usize {
    crypto_kem_mlkem768_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    unsafe { _sodium_mlkem768_ref_seed_keypair(pk, sk, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    unsafe { _sodium_mlkem768_ref_keypair(pk, sk) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    unsafe { _sodium_mlkem768_ref_enc(ct, ss, pk) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    unsafe { _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    unsafe { _sodium_mlkem768_ref_dec(ss, ct, sk) }
}
