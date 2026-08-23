//! Translation of `crypto_generichash/blake2b/generichash_blake2.c`
//! (the `crypto_generichash_blake2b_*` constant accessors).

use core::ffi::c_void;

use crate::randombytes::randombytes_buf;

use super::generichash_blake2b::crypto_generichash_blake2b_state;

// Constants from include/sodium/crypto_generichash_blake2b.h
pub const crypto_generichash_blake2b_BYTES_MIN: usize = 16;
pub const crypto_generichash_blake2b_BYTES_MAX: usize = 64;
pub const crypto_generichash_blake2b_BYTES: usize = 32;
pub const crypto_generichash_blake2b_KEYBYTES_MIN: usize = 16;
pub const crypto_generichash_blake2b_KEYBYTES_MAX: usize = 64;
pub const crypto_generichash_blake2b_KEYBYTES: usize = 32;
pub const crypto_generichash_blake2b_SALTBYTES: usize = 16;
pub const crypto_generichash_blake2b_PERSONALBYTES: usize = 16;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_bytes_min() -> usize {
    crypto_generichash_blake2b_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_bytes_max() -> usize {
    crypto_generichash_blake2b_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_bytes() -> usize {
    crypto_generichash_blake2b_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_keybytes_min() -> usize {
    crypto_generichash_blake2b_KEYBYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_keybytes_max() -> usize {
    crypto_generichash_blake2b_KEYBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_keybytes() -> usize {
    crypto_generichash_blake2b_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_saltbytes() -> usize {
    crypto_generichash_blake2b_SALTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_personalbytes() -> usize {
    crypto_generichash_blake2b_PERSONALBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_blake2b_state>() + 63usize) & !63usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_generichash_blake2b_KEYBYTES);
}
