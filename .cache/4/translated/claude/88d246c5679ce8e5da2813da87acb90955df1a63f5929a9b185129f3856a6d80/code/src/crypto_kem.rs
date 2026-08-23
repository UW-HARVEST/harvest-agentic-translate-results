//! Translation of `c_src/libsodium/crypto_kem/crypto_kem.c`.
//!
//! Constants come from `include/sodium/crypto_kem.h`:
//!   `crypto_kem_PUBLICKEYBYTES    == crypto_kem_xwing_PUBLICKEYBYTES    == 1216U`
//!   `crypto_kem_SECRETKEYBYTES    == crypto_kem_xwing_SECRETKEYBYTES    == 32U`
//!   `crypto_kem_CIPHERTEXTBYTES   == crypto_kem_xwing_CIPHERTEXTBYTES   == 1120U`
//!   `crypto_kem_SHAREDSECRETBYTES == crypto_kem_xwing_SHAREDSECRETBYTES == 32U`
//!   `crypto_kem_SEEDBYTES         == crypto_kem_xwing_SEEDBYTES         == 32U`
//!   `crypto_kem_PRIMITIVE         == "xwing"`

use core::ffi::{c_char, c_int};

const crypto_kem_PUBLICKEYBYTES: usize = 1216;
const crypto_kem_SECRETKEYBYTES: usize = 32;
const crypto_kem_CIPHERTEXTBYTES: usize = 1120;
const crypto_kem_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_SEEDBYTES: usize = 32;
const crypto_kem_PRIMITIVE: &[u8] = b"xwing\0";

extern "C" {
    /* crypto_kem/xwing/kem_xwing.c */
    fn crypto_kem_xwing_seed_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) -> c_int;
    fn crypto_kem_xwing_keypair(pk: *mut u8, sk: *mut u8) -> c_int;
    fn crypto_kem_xwing_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int;
    fn crypto_kem_xwing_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_publickeybytes() -> usize {
    crypto_kem_PUBLICKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_secretkeybytes() -> usize {
    crypto_kem_SECRETKEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_ciphertextbytes() -> usize {
    crypto_kem_CIPHERTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_sharedsecretbytes() -> usize {
    crypto_kem_SHAREDSECRETBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_seedbytes() -> usize {
    crypto_kem_SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_primitive() -> *const c_char {
    crypto_kem_PRIMITIVE.as_ptr() as *const c_char
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
