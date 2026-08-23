pub mod sha256;
pub mod sha3;
pub mod sha512;

// ---------------------------------------------------------------------------
// Translation of `crypto_hash/crypto_hash.c`
// ---------------------------------------------------------------------------

use core::ffi::{c_char, c_int};

/// `#define crypto_hash_BYTES crypto_hash_sha512_BYTES`
pub const crypto_hash_BYTES: usize = self::sha512::crypto_hash_sha512_BYTES;

/// `#define crypto_hash_PRIMITIVE "sha512"`
pub const crypto_hash_PRIMITIVE: &[u8; 7] = b"sha512\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_bytes() -> usize {
    crypto_hash_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash(out: *mut u8, in_: *const u8, inlen: u64) -> c_int {
    unsafe { self::sha512::crypto_hash_sha512(out, in_, inlen) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_primitive() -> *const c_char {
    crypto_hash_PRIMITIVE.as_ptr() as *const c_char
}
