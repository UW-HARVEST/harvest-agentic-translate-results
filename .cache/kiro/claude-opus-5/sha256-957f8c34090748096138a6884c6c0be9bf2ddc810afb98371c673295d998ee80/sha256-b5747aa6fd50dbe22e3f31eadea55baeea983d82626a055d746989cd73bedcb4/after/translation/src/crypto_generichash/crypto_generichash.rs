//! Translation of c_src/libsodium/crypto_generichash/crypto_generichash.c

use core::ffi::{c_char, c_int, c_void};

// crypto_generichash constants alias the blake2b ones (see crypto_generichash.h).
const crypto_generichash_BYTES_MIN: usize = 16;
const crypto_generichash_BYTES_MAX: usize = 64;
const crypto_generichash_BYTES: usize = 32;
const crypto_generichash_KEYBYTES_MIN: usize = 16;
const crypto_generichash_KEYBYTES_MAX: usize = 64;
const crypto_generichash_KEYBYTES: usize = 32;
// #define crypto_generichash_PRIMITIVE "blake2b"
const crypto_generichash_PRIMITIVE: &[u8] = b"blake2b\0";

// crypto_generichash_state == crypto_generichash_blake2b_state (packed, CRYPTO_ALIGN(64)).
#[repr(C, align(64))]
struct crypto_generichash_state {
    opaque: [u8; 384],
}

// crypto_generichash_blake2b_state — same layout, distinct cast target.
#[repr(C, align(64))]
struct crypto_generichash_blake2b_state {
    opaque: [u8; 384],
}

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
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
    crypto_generichash_blake2b(out, outlen, in_, inlen, key, keylen)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_init(
    state: *mut crypto_generichash_state,
    key: *const u8,
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
    in_: *const u8,
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
    out: *mut u8,
    outlen: usize,
) -> c_int {
    crypto_generichash_blake2b_final(
        state as *mut crypto_generichash_blake2b_state,
        out,
        outlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_generichash_KEYBYTES);
}
