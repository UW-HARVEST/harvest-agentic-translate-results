//! Translation of `crypto_stream/crypto_stream.c`

use crate::common::*;
use core::ffi::{c_char, c_int, c_ulonglong, c_void};

extern "C" {
    fn crypto_stream_xsalsa20(
        c: *mut u8,
        clen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: c_ulonglong,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const crypto_stream_KEYBYTES: usize = 32; /* crypto_stream_xsalsa20_KEYBYTES */
const crypto_stream_NONCEBYTES: usize = 24; /* crypto_stream_xsalsa20_NONCEBYTES */

/// `#define crypto_stream_PRIMITIVE "xsalsa20"`
static crypto_stream_PRIMITIVE: [u8; 9] = *b"xsalsa20\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_keybytes() -> usize {
    crypto_stream_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_noncebytes() -> usize {
    crypto_stream_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_primitive() -> *const c_char {
    crypto_stream_PRIMITIVE.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream(
    c: *mut u8,
    clen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20(c, clen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_xor(
    c: *mut u8,
    m: *const u8,
    mlen: c_ulonglong,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_KEYBYTES);
}
