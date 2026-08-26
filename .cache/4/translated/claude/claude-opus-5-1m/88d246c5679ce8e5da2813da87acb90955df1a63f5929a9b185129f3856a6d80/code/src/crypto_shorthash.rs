//! Translation of `crypto_shorthash/crypto_shorthash.c`.

use core::ffi::{c_char, c_int, c_ulonglong, c_void};

extern "C" {
    fn crypto_shorthash_siphash24(
        out: *mut u8,
        in_: *const u8,
        inlen: c_ulonglong,
        k: *const u8,
    ) -> c_int;

    fn randombytes_buf(buf: *mut c_void, size: usize);
}

/* #define crypto_shorthash_BYTES crypto_shorthash_siphash24_BYTES  (8U) */
const crypto_shorthash_BYTES: usize = 8;
/* #define crypto_shorthash_KEYBYTES crypto_shorthash_siphash24_KEYBYTES  (16U) */
const crypto_shorthash_KEYBYTES: usize = 16;
/* #define crypto_shorthash_PRIMITIVE "siphash24" */
static crypto_shorthash_PRIMITIVE: [u8; 10] = *b"siphash24\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_bytes() -> usize {
    crypto_shorthash_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_keybytes() -> usize {
    crypto_shorthash_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_primitive() -> *const c_char {
    crypto_shorthash_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
    k: *const u8,
) -> c_int {
    crypto_shorthash_siphash24(out, in_, inlen, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_shorthash_KEYBYTES);
}
