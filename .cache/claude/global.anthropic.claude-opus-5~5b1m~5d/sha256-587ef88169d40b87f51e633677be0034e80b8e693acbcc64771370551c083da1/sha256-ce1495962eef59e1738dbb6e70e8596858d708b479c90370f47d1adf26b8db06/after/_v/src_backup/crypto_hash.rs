//! Translation of `crypto_hash/crypto_hash.c`.
#![allow(dead_code)]

use core::ffi::{c_char, c_int};

extern "C" {
    fn crypto_hash_sha512(out: *mut u8, inp: *const u8, inlen: u64) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_bytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash(out: *mut u8, inp: *const u8, inlen: u64) -> c_int {
    crypto_hash_sha512(out, inp, inlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_primitive() -> *const c_char {
    static PRIMITIVE: &[u8] = b"sha512\0";
    PRIMITIVE.as_ptr() as *const c_char
}
