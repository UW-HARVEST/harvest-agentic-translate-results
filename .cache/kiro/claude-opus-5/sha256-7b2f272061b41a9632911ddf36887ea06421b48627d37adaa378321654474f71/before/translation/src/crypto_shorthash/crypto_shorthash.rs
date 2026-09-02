//! Translation of c_src/libsodium/crypto_shorthash/crypto_shorthash.c

use core::ffi::{c_char, c_int, c_void};

// crypto_shorthash_BYTES == crypto_shorthash_siphash24_BYTES == 8U
const CRYPTO_SHORTHASH_BYTES: usize = 8;
// crypto_shorthash_KEYBYTES == crypto_shorthash_siphash24_KEYBYTES == 16U
const CRYPTO_SHORTHASH_KEYBYTES: usize = 16;
// crypto_shorthash_PRIMITIVE
const CRYPTO_SHORTHASH_PRIMITIVE: &[u8] = b"siphash24\0";

extern "C" {
    fn crypto_shorthash_siphash24(
        out: *mut u8,
        in_: *const u8,
        inlen: u64,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_bytes() -> usize {
    CRYPTO_SHORTHASH_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_keybytes() -> usize {
    CRYPTO_SHORTHASH_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_primitive() -> *const c_char {
    CRYPTO_SHORTHASH_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_shorthash_siphash24(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_SHORTHASH_KEYBYTES);
}
