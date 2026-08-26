pub mod hmacsha256;
pub mod hmacsha512;
pub mod hmacsha512256;

// Translation of `crypto_auth/crypto_auth.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;

use self::hmacsha512256::crypto_auth_hmacsha512256_state;

/// `#define crypto_auth_BYTES crypto_auth_hmacsha512256_BYTES`
pub const crypto_auth_BYTES: usize = hmacsha512256::crypto_auth_hmacsha512256_BYTES;

/// `#define crypto_auth_KEYBYTES crypto_auth_hmacsha512256_KEYBYTES`
pub const crypto_auth_KEYBYTES: usize = hmacsha512256::crypto_auth_hmacsha512256_KEYBYTES;

/// `#define crypto_auth_PRIMITIVE "hmacsha512256"`
pub const crypto_auth_PRIMITIVE: &[u8; 14] = b"hmacsha512256\0";

/// `typedef crypto_auth_hmacsha512256_state crypto_auth_state;` (not exposed by
/// the public header, kept for documentation purposes).
pub type crypto_auth_state = crypto_auth_hmacsha512256_state;

unsafe extern "C" {
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
    unsafe { crypto_auth_hmacsha512256(out, in_, inlen, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_verify(
    h: *const u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    unsafe { crypto_auth_hmacsha512256_verify(h, in_, inlen, k) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_auth_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_auth_KEYBYTES);
}
