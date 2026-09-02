//! Translation of c_src/libsodium/crypto_auth/crypto_auth.c

use core::ffi::{c_char, c_int};

// #define crypto_auth_BYTES crypto_auth_hmacsha512256_BYTES == 32U
const crypto_auth_BYTES: usize = 32;
// #define crypto_auth_KEYBYTES crypto_auth_hmacsha512256_KEYBYTES == 32U
const crypto_auth_KEYBYTES: usize = 32;
// #define crypto_auth_PRIMITIVE "hmacsha512256"
static crypto_auth_PRIMITIVE: &[u8] = b"hmacsha512256\0";

extern "C" {
    fn crypto_auth_hmacsha512256(
        out: *mut u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn crypto_auth_hmacsha512256_verify(
        h: *const u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_bytes() -> usize {
    crypto_auth_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_keybytes() -> usize {
    crypto_auth_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_primitive() -> *const c_char {
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
