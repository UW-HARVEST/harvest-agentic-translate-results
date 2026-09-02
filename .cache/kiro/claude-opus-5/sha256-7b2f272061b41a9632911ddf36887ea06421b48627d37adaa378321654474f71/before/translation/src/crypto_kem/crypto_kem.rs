//! Translation of c_src/libsodium/crypto_kem/crypto_kem.c

use core::ffi::c_char;

// crypto_kem_PUBLICKEYBYTES == crypto_kem_xwing_PUBLICKEYBYTES
const CRYPTO_KEM_PUBLICKEYBYTES: usize = 1216;
// crypto_kem_SECRETKEYBYTES == crypto_kem_xwing_SECRETKEYBYTES
const CRYPTO_KEM_SECRETKEYBYTES: usize = 32;
// crypto_kem_CIPHERTEXTBYTES == crypto_kem_xwing_CIPHERTEXTBYTES
const CRYPTO_KEM_CIPHERTEXTBYTES: usize = 1120;
// crypto_kem_SHAREDSECRETBYTES == crypto_kem_xwing_SHAREDSECRETBYTES
const CRYPTO_KEM_SHAREDSECRETBYTES: usize = 32;
// crypto_kem_SEEDBYTES == crypto_kem_xwing_SEEDBYTES
const CRYPTO_KEM_SEEDBYTES: usize = 32;
// crypto_kem_PRIMITIVE
const CRYPTO_KEM_PRIMITIVE: &[u8] = b"xwing\0";

extern "C" {
    fn crypto_kem_xwing_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn crypto_kem_xwing_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int;
    fn crypto_kem_xwing_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;
}

use core::ffi::c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_publickeybytes() -> usize {
    CRYPTO_KEM_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_secretkeybytes() -> usize {
    CRYPTO_KEM_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_ciphertextbytes() -> usize {
    CRYPTO_KEM_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_sharedsecretbytes() -> usize {
    CRYPTO_KEM_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_seedbytes() -> usize {
    CRYPTO_KEM_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_primitive() -> *const c_char {
    CRYPTO_KEM_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    crypto_kem_xwing_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    crypto_kem_xwing_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
    crypto_kem_xwing_enc(ct, ss, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int {
    crypto_kem_xwing_dec(ss, ct, sk)
}
