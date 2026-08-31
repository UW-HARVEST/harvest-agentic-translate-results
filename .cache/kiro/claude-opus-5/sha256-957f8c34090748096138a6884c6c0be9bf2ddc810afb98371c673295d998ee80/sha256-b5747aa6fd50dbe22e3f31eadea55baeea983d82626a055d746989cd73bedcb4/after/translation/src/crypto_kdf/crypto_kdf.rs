//! Translation of c_src/libsodium/crypto_kdf/crypto_kdf.c

use core::ffi::{c_char, c_int, c_void};

// crypto_kdf constants alias the blake2b ones (see crypto_kdf.h / crypto_kdf_blake2b.h).
const crypto_kdf_BYTES_MIN: usize = 16;
const crypto_kdf_BYTES_MAX: usize = 64;
const crypto_kdf_CONTEXTBYTES: usize = 8;
const crypto_kdf_KEYBYTES: usize = 32;
// #define crypto_kdf_PRIMITIVE "blake2b"
const crypto_kdf_PRIMITIVE: &[u8] = b"blake2b\0";

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn crypto_kdf_blake2b_derive_from_key(
        subkey: *mut u8,
        subkey_len: usize,
        subkey_id: u64,
        ctx: *const c_char,
        key: *const u8,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_primitive() -> *const c_char {
    crypto_kdf_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_bytes_min() -> usize {
    crypto_kdf_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_bytes_max() -> usize {
    crypto_kdf_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_contextbytes() -> usize {
    crypto_kdf_CONTEXTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_keybytes() -> usize {
    crypto_kdf_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_derive_from_key(
    subkey: *mut u8,
    subkey_len: usize,
    subkey_id: u64,
    ctx: *const c_char,
    key: *const u8,
) -> c_int {
    crypto_kdf_blake2b_derive_from_key(subkey, subkey_len, subkey_id, ctx, key)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kdf_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_kdf_KEYBYTES);
}
