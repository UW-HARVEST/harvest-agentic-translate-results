//! Translation of `crypto_stream/crypto_stream.c`.
//!
//! `crypto_stream` is a thin alias over the `xsalsa20` primitive (the
//! library's default `crypto_stream_*` choice).

use crate::common::SODIUM_SIZE_MAX;
use core::ffi::{c_char, c_int};

extern "C" {
    fn crypto_stream_xsalsa20(c: *mut u8, clen: u64, n: *const u8, k: *const u8) -> c_int;
    fn crypto_stream_xsalsa20_xor(
        c: *mut u8,
        m: *const u8,
        mlen: u64,
        n: *const u8,
        k: *const u8,
    ) -> c_int;
    fn randombytes_buf(buf: *mut u8, size: usize);
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_keybytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_noncebytes() -> usize {
    24
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_primitive() -> *const c_char {
    b"xsalsa20\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream(
    c: *mut u8,
    clen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20(c, clen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_xor(
    c: *mut u8,
    m: *const u8,
    mlen: u64,
    n: *const u8,
    k: *const u8,
) -> c_int {
    crypto_stream_xsalsa20_xor(c, m, mlen, n, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_stream_keygen(k: *mut u8) {
    randombytes_buf(k, 32);
}
