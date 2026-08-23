//! Translation of `c_src/libsodium/crypto_kem/mlkem768/kem_mlkem768.c`.
//!
//! Thin dispatch layer on top of the `mlkem768_ref` implementation.
//! Constants come from `include/sodium/crypto_kem_mlkem768.h`.

use core::ffi::c_int;

const crypto_kem_mlkem768_PUBLICKEYBYTES: usize = 1184;
const crypto_kem_mlkem768_SECRETKEYBYTES: usize = 2400;
const crypto_kem_mlkem768_CIPHERTEXTBYTES: usize = 1088;
const crypto_kem_mlkem768_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_mlkem768_SEEDBYTES: usize = 64;

extern "C" {
    /* crypto_kem/mlkem768/ref/kem_mlkem768_ref.c
     * (names after the `private/quirks.h` renaming) */
    fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn _sodium_mlkem768_ref_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn _sodium_mlkem768_ref_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int;
    fn _sodium_mlkem768_ref_enc_deterministic(
        ct: *mut u8,
        ss: *mut u8,
        pk: *const u8,
        seed: *const u8,
    ) -> c_int;
    fn _sodium_mlkem768_ref_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;
}

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
    _sodium_mlkem768_ref_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    _sodium_mlkem768_ref_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    _sodium_mlkem768_ref_enc(ct, ss, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    _sodium_mlkem768_ref_dec(ss, ct, sk)
}
