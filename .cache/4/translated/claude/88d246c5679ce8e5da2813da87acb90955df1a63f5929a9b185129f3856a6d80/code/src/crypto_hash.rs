//! Translation of `crypto_hash/crypto_hash.c`.

use core::ffi::{c_char, c_int, c_ulonglong};

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: c_ulonglong) -> c_int;
}

/* #define crypto_hash_BYTES crypto_hash_sha512_BYTES  (== 64U) */
const crypto_hash_BYTES: usize = 64;

/* #define crypto_hash_PRIMITIVE "sha512" */
static crypto_hash_PRIMITIVE: [u8; 7] = *b"sha512\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_bytes() -> usize {
    crypto_hash_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    crypto_hash_sha512(out, in_, inlen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_primitive() -> *const c_char {
    crypto_hash_PRIMITIVE.as_ptr() as *const c_char
}
