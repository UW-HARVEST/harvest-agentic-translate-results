pub mod blake2b;
pub mod hkdf_sha256;
pub mod hkdf_sha512;

// Translation of crypto_kdf/crypto_kdf.c and include/sodium/crypto_kdf.h

use core::ffi::{c_char, c_int, c_uchar};

use crate::crypto_kdf::blake2b::{
    crypto_kdf_blake2b_BYTES_MAX, crypto_kdf_blake2b_BYTES_MIN,
    crypto_kdf_blake2b_CONTEXTBYTES, crypto_kdf_blake2b_KEYBYTES,
    crypto_kdf_blake2b_derive_from_key,
};
use crate::randombytes::randombytes_buf;

pub const crypto_kdf_BYTES_MIN: usize = crypto_kdf_blake2b_BYTES_MIN;
pub const crypto_kdf_BYTES_MAX: usize = crypto_kdf_blake2b_BYTES_MAX;
pub const crypto_kdf_CONTEXTBYTES: usize = crypto_kdf_blake2b_CONTEXTBYTES;
pub const crypto_kdf_KEYBYTES: usize = crypto_kdf_blake2b_KEYBYTES;
pub const crypto_kdf_PRIMITIVE: &[u8] = b"blake2b\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_primitive() -> *const c_char {
    crypto_kdf_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_bytes_min() -> usize {
    crypto_kdf_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_bytes_max() -> usize {
    crypto_kdf_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_contextbytes() -> usize {
    crypto_kdf_CONTEXTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kdf_keybytes() -> usize {
    crypto_kdf_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_derive_from_key(
    subkey: *mut c_uchar,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const c_uchar,
) -> c_int {
    crypto_kdf_blake2b_derive_from_key(subkey, subkey_len, subkey_id, ctx, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_keygen(k: *mut c_uchar) {
    randombytes_buf(k as *mut core::ffi::c_void, crypto_kdf_KEYBYTES);
}
