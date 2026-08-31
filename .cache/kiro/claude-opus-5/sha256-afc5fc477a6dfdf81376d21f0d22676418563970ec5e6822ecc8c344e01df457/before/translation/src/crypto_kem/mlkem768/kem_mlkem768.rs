//! Translation of c_src/libsodium/crypto_kem/mlkem768/kem_mlkem768.c

use core::ffi::c_int;

const CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES: usize = 1184;
const CRYPTO_KEM_MLKEM768_SECRETKEYBYTES: usize = 2400;
const CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES: usize = 1088;
const CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES: usize = 32;
const CRYPTO_KEM_MLKEM768_SEEDBYTES: usize = 64;

// quirks.h renames: mlkem768_ref_* -> _sodium_mlkem768_ref_*
extern "C" {
    fn _sodium_mlkem768_ref_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
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
    CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_secretkeybytes() -> usize {
    CRYPTO_KEM_MLKEM768_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_ciphertextbytes() -> usize {
    CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_sharedsecretbytes() -> usize {
    CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_seedbytes() -> usize {
    CRYPTO_KEM_MLKEM768_SEEDBYTES
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
