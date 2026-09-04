pub mod siphash24;

use core::ffi::{c_char, c_int};

use crate::crypto_shorthash::siphash24::{
    crypto_shorthash_siphash24, crypto_shorthash_siphash24_BYTES,
    crypto_shorthash_siphash24_KEYBYTES,
};
use crate::randombytes::randombytes_buf;

pub const crypto_shorthash_BYTES: usize = crypto_shorthash_siphash24_BYTES;
pub const crypto_shorthash_KEYBYTES: usize = crypto_shorthash_siphash24_KEYBYTES;
pub const crypto_shorthash_PRIMITIVE: &[u8] = b"siphash24\0";

#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_bytes() -> usize {
    crypto_shorthash_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_keybytes() -> usize {
    crypto_shorthash_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_primitive() -> *const c_char {
    crypto_shorthash_PRIMITIVE.as_ptr() as *const c_char
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
    randombytes_buf(k as *mut core::ffi::c_void, crypto_shorthash_KEYBYTES);
}
