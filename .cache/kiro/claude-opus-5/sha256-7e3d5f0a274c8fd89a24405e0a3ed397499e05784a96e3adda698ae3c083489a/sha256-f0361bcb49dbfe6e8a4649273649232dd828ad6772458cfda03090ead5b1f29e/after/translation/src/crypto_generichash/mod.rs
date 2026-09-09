pub mod blake2b;

// Translation of:
//   - crypto_generichash/crypto_generichash.c
//   - include/sodium/crypto_generichash.h

use core::ffi::{c_char, c_int, c_uchar, c_void};

use crate::randombytes::randombytes_buf;

use self::blake2b::{
    crypto_generichash_blake2b, crypto_generichash_blake2b_final,
    crypto_generichash_blake2b_init, crypto_generichash_blake2b_state,
    crypto_generichash_blake2b_update,
};

/* ---- crypto_generichash.h constants ---- */
pub const crypto_generichash_BYTES_MIN: usize = blake2b::crypto_generichash_blake2b_BYTES_MIN;
pub const crypto_generichash_BYTES_MAX: usize = blake2b::crypto_generichash_blake2b_BYTES_MAX;
pub const crypto_generichash_BYTES: usize = blake2b::crypto_generichash_blake2b_BYTES;
pub const crypto_generichash_KEYBYTES_MIN: usize =
    blake2b::crypto_generichash_blake2b_KEYBYTES_MIN;
pub const crypto_generichash_KEYBYTES_MAX: usize =
    blake2b::crypto_generichash_blake2b_KEYBYTES_MAX;
pub const crypto_generichash_KEYBYTES: usize = blake2b::crypto_generichash_blake2b_KEYBYTES;
pub const crypto_generichash_PRIMITIVE: &[u8] = b"blake2b\0";

/* typedef crypto_generichash_blake2b_state crypto_generichash_state; */
pub type crypto_generichash_state = crypto_generichash_blake2b_state;

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_bytes_min() -> usize {
    crypto_generichash_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_bytes_max() -> usize {
    crypto_generichash_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_bytes() -> usize {
    crypto_generichash_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_keybytes_min() -> usize {
    crypto_generichash_KEYBYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_keybytes_max() -> usize {
    crypto_generichash_KEYBYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_keybytes() -> usize {
    crypto_generichash_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_primitive() -> *const c_char {
    crypto_generichash_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_state>() + 63usize) & !63usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash(
    out: *mut c_uchar,
    outlen: usize,
    in_: *const c_uchar,
    inlen: u64,
    key: *const c_uchar,
    keylen: usize,
) -> c_int {
    crypto_generichash_blake2b(out, outlen, in_, inlen, key, keylen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_init(
    state: *mut crypto_generichash_state,
    key: *const c_uchar,
    keylen: usize,
    outlen: usize,
) -> c_int {
    crypto_generichash_blake2b_init(
        state as *mut crypto_generichash_blake2b_state,
        key,
        keylen,
        outlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_update(
    state: *mut crypto_generichash_state,
    in_: *const c_uchar,
    inlen: u64,
) -> c_int {
    crypto_generichash_blake2b_update(
        state as *mut crypto_generichash_blake2b_state,
        in_,
        inlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_final(
    state: *mut crypto_generichash_state,
    out: *mut c_uchar,
    outlen: usize,
) -> c_int {
    crypto_generichash_blake2b_final(
        state as *mut crypto_generichash_blake2b_state,
        out,
        outlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keygen(k: *mut c_uchar) {
    randombytes_buf(k as *mut c_void, crypto_generichash_KEYBYTES);
}
