pub mod hmacsha256;
pub mod hmacsha512;
pub mod hmacsha512256;

// Translation of crypto_auth/crypto_auth.c and include/sodium/crypto_auth.h

use core::ffi::{c_char, c_int};

use crate::crypto_auth::hmacsha512256::{
    crypto_auth_hmacsha512256, crypto_auth_hmacsha512256_BYTES, crypto_auth_hmacsha512256_KEYBYTES,
    crypto_auth_hmacsha512256_verify,
};
use crate::randombytes::randombytes_buf;

pub const crypto_auth_BYTES: usize = crypto_auth_hmacsha512256_BYTES;
pub const crypto_auth_KEYBYTES: usize = crypto_auth_hmacsha512256_KEYBYTES;
pub const crypto_auth_PRIMITIVE: &[u8] = b"hmacsha512256\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_bytes() -> usize {
    crypto_auth_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_keybytes() -> usize {
    crypto_auth_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_auth_primitive() -> *const c_char {
    crypto_auth_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_auth_hmacsha512256(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_auth_hmacsha512256_verify(h, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_auth_KEYBYTES);
}
