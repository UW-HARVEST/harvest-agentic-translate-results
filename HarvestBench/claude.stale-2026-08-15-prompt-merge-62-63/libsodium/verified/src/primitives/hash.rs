//! Translated from crypto_hash/crypto_hash.c
use core::ffi::c_char;

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, input: *const u8, inlen: u64) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_bytes() -> usize {
    64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash(out: *mut u8, input: *const u8, inlen: u64) -> i32 {
    crypto_hash_sha512(out, input, inlen)
}

static HASH_PRIMITIVE: &[u8] = b"sha512\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_primitive() -> *const c_char {
    HASH_PRIMITIVE.as_ptr() as *const c_char
}
