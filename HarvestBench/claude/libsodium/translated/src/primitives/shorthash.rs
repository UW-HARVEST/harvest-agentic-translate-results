//! Translated from crypto_shorthash/crypto_shorthash.c
use crate::primitives::cutil::*;
use core::ffi::{c_char, c_void};

extern "C" {
    fn crypto_shorthash_siphash24(out: *mut u8, input: *const u8, inlen: u64, k: *const u8) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_bytes() -> usize {
    8
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_keybytes() -> usize {
    16
}

static SHORTHASH_PRIMITIVE: &[u8] = b"siphash24\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_primitive() -> *const c_char {
    SHORTHASH_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    crypto_shorthash_siphash24(out, input, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, 16);
}
