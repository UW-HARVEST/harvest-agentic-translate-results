//! Translation of `crypto_stream/salsa208/stream_salsa208.c`

use crate::common::*;
use core::ffi::c_void;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

const crypto_stream_salsa208_KEYBYTES: usize = 32;
const crypto_stream_salsa208_NONCEBYTES: usize = 8;

/* LCOV_EXCL_START */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keybytes() -> usize {
    crypto_stream_salsa208_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_noncebytes() -> usize {
    crypto_stream_salsa208_NONCEBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_messagebytes_max() -> usize {
    SODIUM_SIZE_MAX as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_stream_salsa208_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, crypto_stream_salsa208_KEYBYTES);
}

/* LCOV_EXCL_STOP */
