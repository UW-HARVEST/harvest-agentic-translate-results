pub mod blake2b;

// Translation of `crypto_generichash/crypto_generichash.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::randombytes::randombytes_buf;

use self::blake2b::generichash_blake2::{
    crypto_generichash_blake2b_BYTES, crypto_generichash_blake2b_BYTES_MAX,
    crypto_generichash_blake2b_BYTES_MIN, crypto_generichash_blake2b_KEYBYTES,
    crypto_generichash_blake2b_KEYBYTES_MAX, crypto_generichash_blake2b_KEYBYTES_MIN,
};
use self::blake2b::generichash_blake2b::crypto_generichash_blake2b_state;

// Constants from include/sodium/crypto_generichash.h
pub const crypto_generichash_BYTES_MIN: usize = crypto_generichash_blake2b_BYTES_MIN;
pub const crypto_generichash_BYTES_MAX: usize = crypto_generichash_blake2b_BYTES_MAX;
pub const crypto_generichash_BYTES: usize = crypto_generichash_blake2b_BYTES;
pub const crypto_generichash_KEYBYTES_MIN: usize = crypto_generichash_blake2b_KEYBYTES_MIN;
pub const crypto_generichash_KEYBYTES_MAX: usize = crypto_generichash_blake2b_KEYBYTES_MAX;
pub const crypto_generichash_KEYBYTES: usize = crypto_generichash_blake2b_KEYBYTES;

/// `#define crypto_generichash_PRIMITIVE "blake2b"`
static crypto_generichash_PRIMITIVE: [u8; 8] = *b"blake2b\0";

/// `typedef crypto_generichash_blake2b_state crypto_generichash_state;`
pub type crypto_generichash_state = crypto_generichash_blake2b_state;

// Defined in crypto_generichash/blake2b/ref/generichash_blake2b.c.
unsafe extern "C" {
    fn crypto_generichash_blake2b(
        out: *mut u8,
        outlen: usize,
        in_: *const u8,
        inlen: u64,
        key: *const u8,
        keylen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_init(
        state: *mut crypto_generichash_blake2b_state,
        key: *const u8,
        keylen: usize,
        outlen: usize,
    ) -> c_int;
    fn crypto_generichash_blake2b_update(
        state: *mut crypto_generichash_blake2b_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_generichash_blake2b_final(
        state: *mut crypto_generichash_blake2b_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_bytes_min() -> usize {
    crypto_generichash_BYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_bytes_max() -> usize {
    crypto_generichash_BYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_bytes() -> usize {
    crypto_generichash_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keybytes_min() -> usize {
    crypto_generichash_KEYBYTES_MIN
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keybytes_max() -> usize {
    crypto_generichash_KEYBYTES_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keybytes() -> usize {
    crypto_generichash_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_primitive() -> *const c_char {
    crypto_generichash_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_state>() + 63usize) & !63usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> c_int {
    unsafe { crypto_generichash_blake2b(out, outlen, in_, inlen, key, keylen) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_init(
    state: *mut crypto_generichash_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
) -> c_int {
    unsafe {
        crypto_generichash_blake2b_init(
            state as *mut crypto_generichash_blake2b_state,
            key,
            keylen,
            outlen,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_update(
    state: *mut crypto_generichash_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    unsafe {
        crypto_generichash_blake2b_update(
            state as *mut crypto_generichash_blake2b_state,
            in_,
            inlen,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_final(
    state: *mut crypto_generichash_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    unsafe {
        crypto_generichash_blake2b_final(
            state as *mut crypto_generichash_blake2b_state,
            out,
            outlen,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_generichash_KEYBYTES);
}
