pub mod mlkem768;
pub mod xwing;

// Translation of crypto_kem/crypto_kem.c (+ include/sodium/crypto_kem.h).

use crate::crypto_kem::xwing::{
    crypto_kem_xwing_dec, crypto_kem_xwing_enc, crypto_kem_xwing_keypair,
    crypto_kem_xwing_seed_keypair,
};

const crypto_kem_PUBLICKEYBYTES: usize = 1216; /* crypto_kem_xwing_PUBLICKEYBYTES */
const crypto_kem_SECRETKEYBYTES: usize = 32; /* crypto_kem_xwing_SECRETKEYBYTES */
const crypto_kem_CIPHERTEXTBYTES: usize = 1120; /* crypto_kem_xwing_CIPHERTEXTBYTES */
const crypto_kem_SHAREDSECRETBYTES: usize = 32; /* crypto_kem_xwing_SHAREDSECRETBYTES */
const crypto_kem_SEEDBYTES: usize = 32; /* crypto_kem_xwing_SEEDBYTES */
const crypto_kem_PRIMITIVE: &[u8] = b"xwing\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_publickeybytes() -> usize {
    crypto_kem_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_secretkeybytes() -> usize {
    crypto_kem_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_ciphertextbytes() -> usize {
    crypto_kem_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_sharedsecretbytes() -> usize {
    crypto_kem_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_seedbytes() -> usize {
    crypto_kem_SEEDBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_primitive() -> *const core::ffi::c_char {
    crypto_kem_PRIMITIVE.as_ptr() as *const core::ffi::c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> i32 {
    crypto_kem_xwing_seed_keypair(pk, sk, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_keypair(pk: *mut u8, sk: *mut u8) -> i32 {
    crypto_kem_xwing_keypair(pk, sk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> i32 {
    crypto_kem_xwing_enc(ct, ss, pk)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> i32 {
    crypto_kem_xwing_dec(ss, ct, sk)
}
